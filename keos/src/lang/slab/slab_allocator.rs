//! Slab allocator implemenation using treiber's stack, a concurrent lock-free
//! and non-blocking stack implementation.
use super::{Palloc, atomic128::AtomicU128};
use abyss::spinlock::SpinLock;
use core::{
    alloc::AllocError,
    ptr::NonNull,
    sync::atomic::{AtomicU64, Ordering},
};

#[inline]
pub fn into_pointer_tag<T>(v: u128) -> (*mut T, u64)
where
    T: Sized,
{
    (
        (v >> 64) as usize as *mut _,
        (v & 0xffff_ffff_ffff_ffff) as u64,
    )
}

#[inline]
pub fn from_pointer_tag<T>(p: *mut T, v: u64) -> u128
where
    T: Sized,
{
    ((p as usize as u128) << 64) | (v as u128)
}

/// The slab allocator.
pub struct SlabAllocator<const BSIZE: usize, const GROW_SIZE: usize> {
    head: AtomicU128,
    stamp: AtomicU64,
    grow_aux: SpinLock<()>,
}

#[doc(hidden)]
#[repr(transparent)]
pub(super) struct Block<'a> {
    next: AtomicU128,
    _l: core::marker::PhantomData<&'a Block<'a>>,
}

#[cfg(feature = "redzone")]
#[inline(never)]
fn verify_redzone_panic(bsize: usize, ptr: usize, _where: &str, dir: &str) {
    panic!(
        "Heap corruption detected on {}! (redzone {})\nbucket: {:?} head: {:x}\n{:?}",
        _where,
        dir,
        bsize,
        ptr,
        DebugArea { ptr, b: bsize }
    );
}

#[cfg(feature = "redzone")]
fn verify_redzone(rz_size: usize, bsize: usize, ptr: usize, _where: &str) {
    unsafe {
        if !core::slice::from_raw_parts((ptr - rz_size) as *mut u8, rz_size)
            .iter()
            .all(|n| *n == RZ)
        {
            verify_redzone_panic(bsize, ptr, _where, "head")
        }

        if !core::slice::from_raw_parts((ptr + bsize) as *mut u8, rz_size)
            .iter()
            .all(|n| *n == RZ)
        {
            verify_redzone_panic(bsize, ptr, _where, "tail")
        }
    }
}

const RZ: u8 = 0x34;

impl<const BSIZE: usize, const GROW_SIZE: usize> SlabAllocator<BSIZE, GROW_SIZE> {
    #[cfg(feature = "redzone")]
    const REDZONE_SIZE: usize = BSIZE;
    #[cfg(feature = "redzone")]
    const BLOCK_SIZE: usize = BSIZE * 3;
    #[cfg(feature = "redzone")]
    const G_SIZE: usize = GROW_SIZE * 3;

    #[cfg(not(feature = "redzone"))]
    const REDZONE_SIZE: usize = 0;
    #[cfg(not(feature = "redzone"))]
    const BLOCK_SIZE: usize = BSIZE;
    #[cfg(not(feature = "redzone"))]
    const G_SIZE: usize = GROW_SIZE;

    /// Create a new slab allocator with slab size T.
    pub(super) const fn new() -> Self {
        Self {
            head: AtomicU128::new(0),
            stamp: AtomicU64::new(0),
            grow_aux: SpinLock::new(()),
        }
    }

    /// Grow the internal blocks by allocating from the physical frame.
    pub(super) unsafe fn grow(&self, allocator: &Palloc) -> Result<(), AllocError> {
        unsafe {
            let base = allocator.allocate(Self::G_SIZE)?.cast::<u8>().as_ptr() as usize;

            #[cfg(feature = "redzone")]
            core::slice::from_raw_parts_mut(base as *mut u8, Self::G_SIZE).fill(RZ);

            for i in (0..Self::G_SIZE).step_by(Self::BLOCK_SIZE) {
                self.dealloc(base + i + Self::REDZONE_SIZE, allocator)
            }
            Ok(())
        }
    }

    /// Deallocate the Block.
    #[inline]
    pub(super) unsafe fn dealloc(&self, ptr: usize, _allocator: &Palloc) {
        unsafe {
            #[cfg(feature = "redzone")]
            verify_redzone(Self::REDZONE_SIZE, BSIZE, ptr, "dealloc");

            let blk = (ptr as *mut Block).as_mut().unwrap();
            let stamp = self.stamp.fetch_add(1, Ordering::Relaxed);
            loop {
                let head = self.head.load(Ordering::Relaxed);
                blk.next.store(head, Ordering::Relaxed);
                let next = from_pointer_tag(blk as *mut Block, stamp);
                if self
                    .head
                    .compare_exchange(head, next, Ordering::Release, Ordering::Relaxed)
                    .is_ok()
                {
                    break;
                }
            }
        }
    }

    /// Allocate a Block from the allocator.
    #[inline]
    pub(super) unsafe fn alloc(&self, allocator: &Palloc) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            loop {
                let head = self.head.load(Ordering::Acquire);
                let (ptr, _) = into_pointer_tag::<Block>(head);
                if ptr.is_null() {
                    let mut o = Ok(());
                    allocator.serialize(&self.grow_aux, || {
                        // Check whether other thread trigger the growth.
                        if into_pointer_tag::<()>(self.head.load(Ordering::Acquire))
                            .0
                            .is_null()
                        {
                            o = self.grow(allocator);
                        }
                    });
                    o?;
                    continue;
                }

                #[cfg(feature = "redzone")]
                verify_redzone(Self::REDZONE_SIZE, BSIZE, ptr as usize, "alloc");

                let next = (*ptr).next.load(Ordering::Relaxed);
                if self
                    .head
                    .compare_exchange(head, next, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    break Ok(NonNull::slice_from_raw_parts(
                        NonNull::new(ptr as *mut u8).ok_or(AllocError)?,
                        BSIZE,
                    ));
                }
            }
        }
    }
}

struct DebugArea {
    ptr: usize,
    b: usize,
}

impl core::fmt::Debug for DebugArea {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fn fmt_line(f: &mut core::fmt::Formatter<'_>, b: usize) {
            let _ = write!(f, "{b:016x} |");
            for i in 0..8 {
                unsafe {
                    let _ = write!(f, " {:02x}", ((b + i) as *const u8).as_ref().unwrap());
                }
            }
            let _ = write!(f, " |");
            for i in 0..8 {
                unsafe {
                    let _ = write!(f, " {:02x}", ((b + i + 8) as *const u8).as_ref().unwrap());
                }
            }
            let _ = writeln!(f);
        }

        let b = self.ptr & !0xf;
        let _ = writeln!(
            f,
            "                 | 00 01 02 03 04 05 06 07 | 08 09 0a 0b 0c 0d 0e 0f"
        );
        let _ = writeln!(
            f,
            "-----------------+-------------------------+------------------------"
        );
        for i in 0..self.b / 0x10 {
            fmt_line(f, b - self.b + 0x10 * i);
        }
        let _ = writeln!(
            f,
            "-----------------+-------------------------+------------------------"
        );
        for i in 0..self.b / 0x10 {
            fmt_line(f, b + 0x10 * i);
        }
        let _ = writeln!(
            f,
            "-----------------+-------------------------+------------------------"
        );
        for i in 0..self.b / 0x10 {
            fmt_line(f, b + self.b + 0x10 * i);
        }
        let _ = writeln!(
            f,
            "-----------------+-------------------------+------------------------"
        );
        Ok(())
    }
}
