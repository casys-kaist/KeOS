//! Slab allocator.
#[allow(dead_code)]
mod atomic128;
mod slab_allocator;

use crate::mm::ContigPages;
use abyss::{addressing::Kva, spinlock::SpinLock};
use core::{
    alloc::{AllocError, Layout},
    ptr::NonNull,
};
use slab_allocator::SlabAllocator;

/// The array of slab allocators with different sizes.
pub struct Allocator {
    /// Slab allocator for Slab64.
    pub s64: SlabAllocator<0x40, 0x1000>,
    /// Slab allocator for Slab128.
    pub s128: SlabAllocator<0x80, 0x1000>,
    /// Slab allocator for Slab256.
    s256: SlabAllocator<0x100, 0x1000>,
    /// Slab allocator for Slab512.
    s512: SlabAllocator<0x200, 0x1000>,
    /// Slab allocator for Slab1024.
    s1024: SlabAllocator<0x400, 0x1000>,
    /// Slab allocator for Slab2048.
    s2048: SlabAllocator<0x800, 0x2000>,
    /// Slab allocator for Slab4096.
    s4096: SlabAllocator<0x1000, 0x4000>,
    /// Slab allocator for Slab8192.
    s8192: SlabAllocator<0x2000, 0x8000>,
    /// Slab allocator for Slab16384.
    s16384: SlabAllocator<0x4000, 0x10000>,
    /// Slab allocator for Slab32768.
    s32768: SlabAllocator<0x8000, 0x20000>,
    /// Slab allocator for Slab65536.
    s65536: SlabAllocator<0x10000, 0x40000>,
    /// Slab allocator for Slab131072.
    s131072: SlabAllocator<0x20000, 0x80000>,
    allocator: Palloc,
}

#[inline]
fn index_from_size(size: usize) -> u32 {
    if size <= 64 {
        0
    } else {
        64 - (size - 1).leading_zeros() - 6
    }
}

macro_rules! dispatch {
    ($self_:expr_2021, $size:expr_2021, |$t:ident| $code:expr_2021) => {{
        match index_from_size($size) {
            0 => {
                let $t = &$self_.s64;
                Ok($code)
            }
            1 => {
                let $t = &$self_.s128;
                Ok($code)
            }
            2 => {
                let $t = &$self_.s256;
                Ok($code)
            }
            3 => {
                let $t = &$self_.s512;
                Ok($code)
            }
            4 => {
                let $t = &$self_.s1024;
                Ok($code)
            }
            5 => {
                let $t = &$self_.s2048;
                Ok($code)
            }
            6 => {
                let $t = &$self_.s4096;
                Ok($code)
            }
            7 => {
                let $t = &$self_.s8192;
                Ok($code)
            }
            8 => {
                let $t = &$self_.s16384;
                Ok($code)
            }
            9 => {
                let $t = &$self_.s32768;
                Ok($code)
            }
            10 => {
                let $t = &$self_.s65536;
                Ok($code)
            }
            11 => {
                let $t = &$self_.s131072;
                Ok($code)
            }
            _ => Err($size),
        }
    }};
}

impl Allocator {
    /// Create a new Allocator.
    const fn new() -> Self {
        Self {
            s64: SlabAllocator::new(),
            s128: SlabAllocator::new(),
            s256: SlabAllocator::new(),
            s512: SlabAllocator::new(),
            s1024: SlabAllocator::new(),
            s2048: SlabAllocator::new(),
            s4096: SlabAllocator::new(),
            s8192: SlabAllocator::new(),
            s16384: SlabAllocator::new(),
            s32768: SlabAllocator::new(),
            s65536: SlabAllocator::new(),
            s131072: SlabAllocator::new(),
            allocator: Palloc,
        }
    }
}

unsafe impl core::alloc::GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        if size == 0 {
            core::ptr::NonNull::dangling().as_ptr()
        } else {
            assert!(
                layout.align() <= size,
                "align: {:?} size: {:?}",
                layout.align(),
                size
            );
            unsafe {
                match dispatch!(self, size, |allocator| allocator.alloc(&self.allocator)) {
                    Ok(o) => o,
                    Err(size) => self.allocator.allocate(size),
                }
                .map(|n| n.as_ptr() as *mut u8)
                .unwrap_or(core::ptr::null_mut())
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            if layout.size() != 0 {
                debug_assert!(layout.align() <= layout.size());
                if let Err(_size) = dispatch!(self, layout.size(), |allocator| allocator
                    .dealloc(ptr as usize, &self.allocator))
                {
                    self.allocator.deallocate(ptr, layout.size());
                }
            }
        }
    }
}

struct Palloc;

impl Palloc {
    unsafe fn allocate(&self, size: usize) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            match crate::mm::ContigPages::new_with_align(size, size) {
                Some(pg) => {
                    let va = pg.kva().into_usize();
                    core::mem::forget(pg);
                    NonNull::new(core::slice::from_raw_parts_mut(va as *mut u8, size))
                        .ok_or(AllocError)
                }
                _ => Err(AllocError),
            }
        }
    }

    unsafe fn deallocate(&self, ptr: *mut u8, size: usize) {
        unsafe {
            ContigPages::from_va(Kva::new(ptr as usize).unwrap(), size);
        }
    }

    fn serialize<F>(&self, aux: &SpinLock<()>, f: F)
    where
        F: FnOnce(),
    {
        let _guard = aux.lock();
        f();
        _guard.unlock();
    }
}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator::new();
