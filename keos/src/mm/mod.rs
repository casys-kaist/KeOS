//! Memory Management.
//!
//! This module implements functionality for memory management operations such
//! as allocating and deallocating memory. The core abstraction is the [`Page`],
//! which represents a single memory page.
//!
//! Memory allocation and deallocation in KeOS is closely tied to Rust's
//! ownership and lifetime system: A page is allocated by creating an instance
//! of the [`Page`] struct. Once the [`Page`] instance is dropped, the page is
//! automatically freed, ensuring proper memory management and preventing memory
//! leaks.
pub mod page_table;
pub mod tlb;

use crate::addressing::{Kva, PAGE_MASK, PAGE_SHIFT, Pa};
use abyss::{boot::Regions, spinlock::SpinLock};
use alloc::vec::Vec;
use core::{
    ops::Range,
    sync::atomic::{AtomicU64, Ordering},
};

/// A reference of a memory page.
///
/// `PageRef` represents a borrowed reference to a kernel virtual address
/// (`Kva`) that maps to a memory.
///
/// # Usage
/// This struct is useful for safely accessing mapped kernel pages without
/// requiring ownership transfers. The lifetime parameter `'a` ensures that
/// the reference does not outlive the memory it points to.
pub struct PageRef<'a> {
    kva: Kva,
    _lt: core::marker::PhantomData<&'a ()>,
}

impl PageRef<'_> {
    /// Build a page refernce from physical address.
    ///
    /// # Safety
    /// [`Pa`] must be held by a other object.
    pub unsafe fn from_pa(pa: Pa) -> Self {
        PageRef {
            kva: pa.into_kva(),
            _lt: core::marker::PhantomData,
        }
    }

    ///  Increase the page reference count corresponding to.
    pub fn into_page(&self) -> Page {
        let page = unsafe { Page::from_pa(self.kva.into_pa()) };
        Page::into_raw(page.clone());
        page
    }

    /// Get the kernel virtual address of this page.
    ///
    /// # Returns
    /// - The kernel virtual address ([`Kva`]) of the page.
    ///
    /// ## Example:
    /// ```rust
    /// let kva = page.kva(); // Get the kernel virtual address
    /// ```
    #[inline]
    pub fn kva(&self) -> Kva {
        self.kva
    }

    /// Get the physical address of this page.
    ///
    /// # Returns
    /// - The physical address ([`Pa`]) of the page.
    ///
    /// ## Example:
    /// ```rust
    /// let pa = page.pa(); // Get the physical address
    /// ```
    #[inline]
    pub fn pa(&self) -> Pa {
        self.kva.into_pa()
    }

    /// Get a reference to the underlying slice of the page (read-only).
    ///
    /// This method allows access to the contents of the page as a byte slice.
    /// The caller can read from the page's memory, but cannot modify it.
    ///
    /// # Returns
    /// - A reference to the byte slice representing the contents of the page.
    ///
    /// ## Example:
    /// ```rust
    /// let slice = page.inner(); // Get read-only access to the page's content
    /// ```
    pub fn inner(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.kva().into_usize() as *const u8, 4096) }
    }

    /// Get a mutable reference to the underlying slice of the page.
    ///
    /// This method allows modification of the contents of the page as a byte
    /// slice. The caller can read from and write to the page's memory.
    ///
    /// # Returns
    /// - A mutable reference to the byte slice representing the contents of the
    ///   page.
    ///
    /// ## Example:
    /// ```rust
    /// let slice = page.inner_mut();
    /// ```
    pub fn inner_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.kva().into_usize() as *mut u8, 4096) }
    }
}

/// A representation of a memory page.
///
/// The [`Page`] struct encapsulates a single memory page, providing methods to
/// allocate, access, and manipulate the underlying page's contents.
///
/// This page internally holds the reference counts. This counter increases on a
/// calling of [`Page::clone`], and decreases when the page instance is dropped.
///
/// ## Example:
/// ```
/// let page = Page::new().unwrap();
/// let va = page.va();  // Get the virtual address of the page.
/// let pa = page.pa();  // Get the physical address of the page.
/// ```
#[derive(Clone)]
pub struct Page {
    inner: ContigPages,
}

impl Default for Page {
    fn default() -> Self {
        Self::new()
    }
}

impl Page {
    /// Allocate a new page.
    ///
    /// This function allocates a new memory page.
    #[inline]
    #[track_caller]
    pub fn new() -> Self {
        let loc = core::panic::Location::caller();
        ContigPages::new(0x1000)
            .map(|inner| Self { inner })
            .inspect(|pg| {
                crate::thread::with_current(|th| {
                    let mut guard = th.allocations.lock();
                    if let Some(alloc) = &mut *guard {
                        assert!(alloc.insert(pg.kva(), loc).is_none())
                    }
                    guard.unlock();
                });
            })
            .expect("Failed to allocate page.")
    }

    /// Get the kernel virtual address of this page.
    ///
    /// # Returns
    /// - The kernel virtual address ([`Kva`]) of the page.
    #[inline]
    pub fn kva(&self) -> Kva {
        self.inner.kva
    }

    /// Get the physical address of this page.
    ///
    /// # Returns
    /// - The physical address ([`Pa`]) of the page.
    #[inline]
    pub fn pa(&self) -> Pa {
        self.inner.kva.into_pa()
    }

    /// Consumes the page, returning its physical address.
    ///
    /// This method "consumes" the [`Page`] and returns its physical address.
    /// After calling this function, the caller is responsible for managing
    /// the memory previously associated with the page. It is important to
    /// properly release the page, which can be done using [`Page::from_pa`].
    ///
    /// # Returns
    /// - The physical address ([`Pa`]) of the page.
    #[inline]
    pub fn into_raw(self) -> Pa {
        core::mem::ManuallyDrop::new(self).pa()
    }

    /// Constructs a page from a given physical address.
    ///
    /// This method reconstructs a [`Page`] from a physical address ([`Pa`]). It
    /// should be used only after consuming a [`Page`] with
    /// [`Page::into_raw`]. The physical address passed must be valid.
    ///
    /// # Safety
    /// - This function is unsafe because incorrect usage could result in memory
    ///   issues such as a double-free.
    /// - Ensure that the physical address passed is valid and is being used
    ///   correctly.
    ///
    /// # Arguments
    /// - `pa`: The physical address of the page.
    ///
    /// # Returns
    /// - A [`Page`] reconstructed from the physical address.
    #[inline]
    pub unsafe fn from_pa(pa: Pa) -> Self {
        Page {
            inner: unsafe { ContigPages::from_va(pa.into_kva(), 0x1000) },
        }
    }

    /// Get a reference to the underlying slice of the page (read-only).
    ///
    /// This method allows access to the contents of the page as a byte slice.
    /// The caller can read from the page's memory, but cannot modify it.
    ///
    /// # Returns
    /// - A reference to the byte slice representing the contents of the page.
    pub fn inner(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.kva().into_usize() as *const u8, 4096) }
    }

    /// Get a mutable reference to the underlying slice of the page.
    ///
    /// This method allows modification of the contents of the page as a byte
    /// slice. The caller can read from and write to the page's memory.
    ///
    /// # Returns
    /// - A mutable reference to the byte slice representing the contents of the
    ///   page.
    pub fn inner_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.kva().into_usize() as *mut u8, 4096) }
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        crate::thread::with_current(|th| {
            let mut guard = th.allocations.lock();
            if let Some(alloc) = &mut *guard {
                assert!(alloc.remove(&self.kva()).is_some());
            }
            guard.unlock();
        });
    }
}

struct BytePP(usize);
impl core::fmt::Display for BytePP {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0 > 16 * 1024 * 1024 * 1024 {
            write!(f, "{} GiB", self.0 / 1024 / 1024 / 1024)
        } else if self.0 > 16 * 1024 * 1024 {
            write!(f, "{} MiB", self.0 / 1024 / 1024)
        } else if self.0 > 16 * 1024 {
            write!(f, "{} KiB", self.0 / 1024)
        } else {
            write!(f, "{} B", self.0)
        }
    }
}

/// Initialize the physical memory allocator.
#[doc(hidden)]
pub unsafe fn init_mm(regions: Regions) {
    unsafe {
        unsafe extern "C" {
            static __edata_end: u64;
        }

        let edata_end = Kva::new(&__edata_end as *const _ as usize).unwrap();

        for region in regions.iter() {
            if region.usable {
                let Range { start, end } = region.addr;
                let (start, end) = (start.into_kva().max(edata_end), end.into_kva());
                if start < end {
                    info!(
                        "    Usable: 0x{:016x}~0x{:016x} ({})",
                        start.into_usize(),
                        end.into_usize(),
                        BytePP(end.into_usize() - start.into_usize())
                    );
                    let mut allocator = PALLOC.lock();
                    allocator.foster(start, end);
                    allocator.unlock();
                }
            }
        }
    }
}

// Physical memory allocators.
struct Arena {
    start: Kva,
    end: Kva,
    // 0: used, 1: unused
    bitmap: &'static mut [u64],
    ref_cnts: &'static [AtomicU64],
}

impl Arena {
    const EMPTY: Option<Self> = None;
    fn set_used(&mut self, index: usize) {
        let (pos, ofs) = (index / 64, index % 64);
        debug_assert_ne!(self.bitmap[pos] & (1 << ofs), 0);
        self.bitmap[pos] &= !(1 << ofs);
        debug_assert_eq!(self.bitmap[pos] & (1 << ofs), 0);
    }
    fn set_unused(&mut self, index: usize) {
        let (pos, ofs) = (index / 64, index % 64);
        debug_assert_eq!(self.bitmap[pos] & (1 << ofs), 0);
        self.bitmap[pos] |= 1 << ofs;
        debug_assert_ne!(self.bitmap[pos] & (1 << ofs), 0);
    }
    fn alloc(&mut self, cnt: usize, align: usize) -> Option<(Kva, &'static AtomicU64)> {
        let mut search = 0;
        while search < self.bitmap.len() * 64 {
            let (mut pos, ofs) = (search / 64, search % 64);
            // search first qword that contains one.
            if ofs % 64 == 0 {
                while self.bitmap[pos] == 0 {
                    pos += 1;
                }
                search = pos * 64;
            }

            let mut cont = 0;
            if align != 0
                && !((self.start.into_usize() >> PAGE_SHIFT) + search).is_multiple_of(align)
            {
                search += 1;
            } else {
                let start = search;
                loop {
                    // Found!
                    if cont == cnt {
                        for i in start..start + cnt {
                            self.set_used(i);
                        }
                        let ref_cnt = &self.ref_cnts[start];
                        debug_assert_eq!(
                            ref_cnt.fetch_add(1, core::sync::atomic::Ordering::SeqCst),
                            0
                        );
                        return Some((self.start + (start << PAGE_SHIFT), ref_cnt));
                    }

                    let (pos, ofs) = (search / 64, search % 64);
                    search += 1;
                    if self.bitmap[pos] & (1 << ofs) != 0 {
                        // usable
                        cont += 1;
                    } else {
                        break;
                    }
                }
            }
        }
        None
    }
    fn dealloc(&mut self, va: Kva, cnt: usize) {
        let ofs = (va.into_usize() - self.start.into_usize()) >> PAGE_SHIFT;
        for i in ofs..ofs + cnt {
            self.set_unused(i);
        }
    }
    fn ref_cnt_for_va(&self, va: Kva) -> &'static AtomicU64 {
        &self.ref_cnts[(va - self.start) >> PAGE_SHIFT]
    }
}

struct PhysicalAllocator {
    inner: [Option<Arena>; 8],
    max_idx: usize,
}

static PALLOC: SpinLock<PhysicalAllocator> = SpinLock::new(PhysicalAllocator {
    inner: [Arena::EMPTY; 8],
    max_idx: 0,
});

impl PhysicalAllocator {
    unsafe fn foster(&mut self, start: Kva, end: Kva) {
        unsafe {
            // Calculate usable page of this region.
            let usable_pages = (end.into_usize() - start.into_usize()) >> PAGE_SHIFT;
            let mut meta_end = start;
            // Each region has alloc bitmap on first N pages.
            let bitmap = core::slice::from_raw_parts_mut(
                start.into_usize() as *mut u64,
                usable_pages.div_ceil(64),
            );
            bitmap.fill(u64::MAX);
            meta_end += 8 * bitmap.len();
            // Array for reference counts are following to the bitmap.
            core::slice::from_raw_parts_mut(meta_end.into_usize() as *mut u64, usable_pages)
                .fill(0);
            let ref_cnts = core::slice::from_raw_parts(
                meta_end.into_usize() as *const AtomicU64,
                usable_pages,
            );
            meta_end += 8 * ref_cnts.len();
            meta_end = (meta_end + PAGE_MASK) & !PAGE_MASK;

            let mut arena = Arena {
                bitmap,
                start,
                end,
                ref_cnts,
            };
            // Pad front.
            for i in 0..(meta_end - start) >> PAGE_SHIFT {
                arena.set_used(i);
            }
            // Pad back.
            for i in usable_pages..((usable_pages + 63) & !63) {
                arena.set_used(i);
            }
            self.inner[self.max_idx] = Some(arena);
            self.max_idx += 1;
        }
    }
}

/// A contiguous pages representation.
pub struct ContigPages {
    arena_idx: usize,
    kva: Kva,
    cnt: usize,
    ref_cnt: &'static AtomicU64,
}

impl Clone for ContigPages {
    fn clone(&self) -> Self {
        self.ref_cnt.fetch_add(1, Ordering::SeqCst);

        Self {
            arena_idx: self.arena_idx,
            kva: self.kva,
            cnt: self.cnt,
            ref_cnt: self.ref_cnt,
        }
    }
}

impl ContigPages {
    /// Allocate a page.
    #[inline]
    pub fn new(size: usize) -> Option<Self> {
        Self::new_with_align(size, 0x1000)
    }

    /// Allocate a page with align
    #[inline]
    pub fn new_with_align(size: usize, align: usize) -> Option<Self> {
        if size != 0 {
            // align up to page size.
            let cnt = (size + PAGE_MASK) >> PAGE_SHIFT;
            let mut allocator = PALLOC.lock();
            let max_idx = allocator.max_idx;
            for (arena_idx, arena) in allocator.inner.iter_mut().take(max_idx).enumerate() {
                if let Some((kva, ref_cnt)) =
                    arena.as_mut().unwrap().alloc(cnt, align >> PAGE_SHIFT)
                {
                    unsafe {
                        core::slice::from_raw_parts_mut(
                            kva.into_usize() as *mut u64,
                            cnt * 0x1000 / core::mem::size_of::<u64>(),
                        )
                        .fill(0);
                    }
                    allocator.unlock();
                    return Some(Self {
                        arena_idx,
                        kva,
                        cnt,
                        ref_cnt,
                    });
                }
            }
        }
        None
    }

    /// Get virtual address of this page.
    #[inline]
    pub fn kva(&self) -> Kva {
        self.kva
    }

    /// Constructs a page from a kva.
    ///
    /// ## Safety
    /// This function is unsafe because improper use may lead to memory
    /// problems. For example, a double-free may occur if the function is called
    /// twice on the same raw pointer.
    #[inline]
    pub unsafe fn from_va(kva: Kva, size: usize) -> Self {
        let allocator = PALLOC.lock();
        let arena_idx = allocator
            .inner
            .iter()
            .take(allocator.max_idx)
            .enumerate()
            .find_map(|(idx, arena)| {
                let Arena { start, end, .. } = arena.as_ref().unwrap();
                if (*start..*end).contains(&kva) {
                    Some(idx)
                } else {
                    None
                }
            })
            .expect("Failed to find arena index.");
        let page = ContigPages {
            arena_idx,
            kva,
            cnt: size / 4096,
            ref_cnt: allocator.inner[arena_idx]
                .as_ref()
                .unwrap()
                .ref_cnt_for_va(kva),
        };
        allocator.unlock();
        page
    }

    /// Split the ContigPages into multiple pages.
    pub fn split(self) -> Vec<Page> {
        let mut out = Vec::new();
        assert_eq!(self.ref_cnt.load(Ordering::SeqCst), 1);
        let this = core::mem::ManuallyDrop::new(self);
        for i in 0..this.cnt {
            let ref_cnt = unsafe {
                &core::slice::from_raw_parts(this.ref_cnt as *const AtomicU64, this.cnt)[i]
            };
            if i != 0 {
                assert_eq!(ref_cnt.fetch_add(1, Ordering::SeqCst), 1);
            }
            out.push(Page {
                inner: ContigPages {
                    arena_idx: this.arena_idx,
                    kva: this.kva + i * 0x1000,
                    cnt: 1,
                    ref_cnt,
                },
            })
        }
        out
    }
}

impl Drop for ContigPages {
    fn drop(&mut self) {
        if self.ref_cnt.fetch_sub(1, Ordering::SeqCst) == 1 {
            let mut allocator = PALLOC.lock();
            allocator.inner[self.arena_idx]
                .as_mut()
                .unwrap()
                .dealloc(self.kva, self.cnt);
            allocator.unlock();
        }
    }
}
