//! # Lazy Paging
//!
//! The lazy paging or demand paging is an another policy for the
//! paging, which used by modern operating systems. Unlink the [`EagerPager`]
//! that you implemented in project 2, the [`LazyPager`] defers physical page
//! allocation until a page fault occurs. This method optimizes memory usage by
//! mapping memory pages **on demand**, rather than preallocating them.
//!
//! Instead of allocating physical memory during the `mmap` call, the OS records
//! **metadata** about the mapping and waits to allocate physical memory until
//! the first **page fault** on that region. When a page fault occurs, the
//! kernel allocates and maps the required physical page.
//! In other words, **page table entries are created only when accessed**.
//!
//! ## Page Fault in KeOS
//!
//! The main function responsible for handling page faults lies in
//! [`Task::page_fault`]. This resolves the page fault reason into
//! [`PageFaultReason`] by reading the `cr2`, which contains faulting address,
//! and decoding the error code on the interrupt stack.
//!
//! It then delegates the page fault handling into the
//! [`LazyPager::handle_page_fault`]. This method is responsible to look up the
//! lazy mapping metadata recorded during the `mmap` and determine whether the
//! fault is bogus fault or not. If the address is valid, it should allocate a
//! new physical page and maps the page into page table. Otherwise, killing the
//! current process by returning the [`KernelError`].
//!
//! ## [`VmAreaStruct`]
//!
//! The [`VmAreaStruct`] represents a range of virtual addresses that share the
//! same memory permissions, similar to the Linux kernel's `struct
//! vm_area_struct`. It serves as the core metadata structure for memory-mapped
//! regions created via `mmap`, capturing the virtual range and the method
//! for populating that region's contents on access.
//!
//! Each [`VmAreaStruct`] is associated with an implementation of the
//! [`MmLoader`] trait, which defines how the contents of a page should be
//! supplied when the region is accessed. This trait-based abstraction
//! enables the kernel to support multiple types of memory mappings in a uniform
//! way. For instance, file-backed mappings use a [`FileBackedLoader`], which
//! reads contents from a file, while anonymous mappings use an [`AnonLoader`],
//! which typically supplies zero-filled pages. Each loader implementation can
//! maintain its own internal state, supporting extensibility and encapsulates
//! the complexity of mapping behavior within each loader.
//!
//! The [`MmLoader`] trait provides a single method, `load`, which is called
//! during demand paging when a page fault occurs at an address within the
//! associated [`VmAreaStruct`]. The method must return a fully initialized
//! [`Page`] object corresponding to that virtual address. The returned page is
//! then mapped into the page table by the pager.
//!
//! This loader-based architecture provides a clean separation of concerns:
//! [`VmAreaStruct`] tracks regions and permissions, while [`MmLoader`]
//! encapsulates how pages are provisioned. This allows KeOS to support flexible
//! and efficient memory models while maintaining clean abstractions.
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`LazyPager`]
//! - [`LazyPager::new`]
//! - [`LazyPager::mmap`]
//! - [`LazyPager::munmap`]
//! - [`LazyPager::get_user_page`]
//! - [`LazyPager::access_ok`]
//! - [`PageFaultReason::is_demand_paging_fault`]
//! - [`LazyPager::do_lazy_load`]
//! - [`VmAreaStruct`]
//! - [`FileBackedLoader`]
//! - [`FileBackedLoader::load`]
//!
//! After implement the functionalities, move on to the next [`section`].
//!
//! [`section`]: mod@crate::fork
//! [`EagerPager`]: ../../keos_project2/mmap/struct.EagerPager.html

use alloc::sync::Arc;
#[cfg(doc)]
use keos::task::Task;
use keos::{
    KernelError,
    addressing::Va,
    fs::RegularFile,
    mm::{Page, PageRef, page_table::Permission},
    task::PFErrorCode,
};
use keos_project2::{page_table::PageTable, pager::Pager};

/// A trait for loading the contents of a virtual memory page on demand.
///
/// This trait abstracts the mechanism for supplying the contents of a page
/// during **demand paging**. It is used by a lazy pager when handling a
/// page fault for a region that has not yet been populated.
///
/// Implementors of this trait can define custom behaviors, such as reading
/// from a file, or zero-filling anonymous pages.
pub trait MmLoader
where
    Self: Send + Sync,
{
    /// Loads and returns the content for the page at the given virtual address.
    ///
    /// The pager will call this function when a page fault occurs at `addr`
    /// within the corresponding [`VmAreaStruct`]. This method must return a
    /// fully initialized [`Page`] containing the data for that virtual page.
    ///
    /// # Parameters
    /// - `addr`: The virtual address of the page to be loaded. This address is
    ///   guaranteed to lie within the virtual memory area associated with this
    ///   loader.
    ///
    /// # Returns
    /// - A newly allocated [`Page`] containing the initialized data for the
    ///   page.
    fn load(&self, addr: Va) -> Page;
}

/// A loader for anonymous memory regions.
///
/// [`AnonLoader`] is used for memory mappings that are not backed by any file.
/// When a page fault occurs, this loader simply returns a newly allocated
/// zero-filled [`Page`].
pub struct AnonLoader {}
impl MmLoader for AnonLoader {
    /// Returns a zero-filled page for the given virtual address.
    ///
    /// Since anonymous memory is not backed by any persistent source, this
    /// implementation always returns a freshly zero-initialized [`Page`].
    fn load(&self, _addr: Va) -> Page {
        Page::new()
    }
}

/// A loader for file-backed memory regions.
///
/// [`FileBackedLoader`] is used for memory mappings backed by files, such as
/// when `mmap` is called with a regular file. This loader reads data from
/// the underlying file starting at a specific offset and returns it in a
/// newly allocated [`Page`].
///
/// The offset within the file is determined based on the virtual address
/// passed to `load`, relative to the mapping’s start address and offset.
///
/// This loader handles partial page reads and fills any unread bytes with
/// zeroes.
pub struct FileBackedLoader {
    // TODO: Define any member you need.
}

impl MmLoader for FileBackedLoader {
    /// Loads a page from the file based on the given virtual address.
    ///
    /// This implementation calculates the offset within the file and reads
    /// up to one page of data into memory. If the read returns fewer than
    /// `PAGE_SIZE` bytes, the remainder of the page is zero-filled.
    fn load(&self, addr: Va) -> Page {
        todo!()
    }
}

/// Represents a memory-mapped region within a process's virtual address space,
/// corresponding to the Linux kernel's `struct vm_area_struct`.
///
/// Each [`VmAreaStruct`] corresponds to a contiguous region with a specific
/// mapping behavior—e.g., anonymous memory or file-backed memory.
/// This abstraction allows the pager to defer the actual page population
/// until the memory is accessed, using demand paging.
///
/// The key component is the [`MmLoader`], which defines how to load
/// the contents of a page when it is accessed (e.g., reading from a file
/// or zero-filling the page).
#[derive(Clone)]
pub struct VmAreaStruct {
    /// A handle to the memory loader for this region.
    ///
    /// The [`MmLoader`] defines how to populate pages in this VMA during
    /// lazy loading. The loader must be thread-safe and cloneable.
    pub loader: Arc<dyn MmLoader>,
    // TODO: Define any member you need.
}

/// The [`LazyPager`] structure implements lazy paging, where memory pages are
/// mapped only when accessed (on page fault), instead of during `mmap` calls.
#[derive(Clone)]
pub struct LazyPager {
    // TODO: Define any member you need.
}

impl Pager for LazyPager {
    /// Creates a new instance of [`LazyPager`].
    ///
    /// This constructor initializes an empty [`LazyPager`] struct.
    fn new() -> Self {
        LazyPager {
            // TODO: Initialize any member you need.
        }
    }

    /// Memory map function (`mmap`) for lazy paging.
    ///
    /// This function creates the metadata for memory mappings, and delegate the
    /// real mappings on page fault.
    ///
    /// Returns an address for the mapped area.
    fn mmap(
        &mut self,
        _page_table: &mut PageTable,
        addr: Va,
        size: usize,
        prot: Permission,
        file: Option<&RegularFile>,
        offset: usize,
    ) -> Result<usize, KernelError> {
        todo!()
    }

    /// Memory unmap function (`munmap`) for lazy paging.
    ///
    /// This function would unmap a previously mapped memory region, releasing
    /// any associated resources.
    ///
    /// # Returns
    /// - Zero (if succeed) or an error ([`KernelError`]).
    fn munmap(&mut self, page_table: &mut PageTable, addr: Va) -> Result<usize, KernelError> {
        todo!()
    }

    /// Find a mapped page at the given virtual address. If the page for addr is
    /// not loaded, load it and then returns.
    ///
    /// This function searches for a memory page mapped at `addr` and, if found,
    /// returns a tuple of [`PageRef`] to the page and its corresponding
    /// [`Permission`] flags.
    ///
    /// # Parameters
    /// - `addr`: The virtual address ([`Va`]) of the page to find.
    ///
    /// # Returns
    /// - `Some(([`PageRef`], [`Permission`]))`: If the page is found.
    /// - `None`: If no mapped page is found at `addr`.
    fn get_user_page(
        &mut self,
        page_table: &mut PageTable,
        addr: Va,
    ) -> Option<(PageRef<'_>, Permission)> {
        todo!()
    }

    /// Checks whether access to the given virtual address is permitted.
    ///
    /// This function verifies that a virtual address `va` is part of a valid
    /// memory mapping and that the requested access type (read or write) is
    /// allowed by the page's protection flags. Note that this does not trigger
    /// the demand paging.
    fn access_ok(&self, va: Va, is_write: bool) -> bool {
        todo!()
    }
}

/// Represents the reason for a page fault in a virtual memory system.
///
/// This struct is used to capture various details about a page fault, including
/// the faulting address, the type of access that caused the fault (read or
/// write).
#[derive(Debug)]
pub struct PageFaultReason {
    /// The address that caused the page fault.
    ///
    /// This is the virtual address that the program attempted to access when
    /// the page fault occurred. It can be useful for debugging and
    /// identifying the location in memory where the fault happened.
    pub fault_addr: Va,

    /// Indicates whether the fault was due to a write access violation.
    ///
    /// A value of `true` means that the program attempted to write to a page
    /// that was marked as read-only or otherwise restricted from write
    /// access. A value of `false` indicates that the access was a read
    /// or the page allowed write access.
    pub is_write_access: bool,

    /// Indicates whether the page that caused the fault is present in memory.
    ///
    /// A value of `true` means that the page is currently loaded into memory,
    /// and the fault may have occurred due to other conditions, such as
    /// protection violations. A value of `false` means the page is not present
    /// in memory (e.g., the page might have been swapped out or mapped as a
    /// non-resident page).
    pub is_present: bool,
}

impl PageFaultReason {
    /// Probe the cause of page fault into a [`PageFaultReason`].
    ///
    /// This function decodes a hardware-provided [`PFErrorCode`],
    /// generated by the CPU when a page fault occurs, into a structured
    /// [`PageFaultReason`] that the kernel can interpret.
    ///
    /// The decoded information includes:
    /// - The type of access (`is_write_access`),
    /// - Whether the faulting page is currently mapped (`is_present`),
    /// - The faulting virtual address (`fault_addr`).
    pub fn new(ec: PFErrorCode, cr2: Va) -> Self {
        PageFaultReason {
            fault_addr: cr2,
            is_write_access: ec.contains(PFErrorCode::WRITE_ACCESS),
            is_present: ec.contains(PFErrorCode::PRESENT),
        }
    }

    /// Returns `true` if the fault is due to **demand paging**.
    ///
    /// # Returns
    /// - `true` if this fault was caused by an demand paging.
    /// - `false` otherwise.
    #[inline]
    pub fn is_demand_paging_fault(&self) -> bool {
        todo!()
    }
}

impl LazyPager {
    /// Handles a page fault by performing **lazy loading** of the faulting
    /// page.
    ///
    /// This method is invoked when a page fault occurs due to **demand
    /// paging**— that is, when a program accesses a virtual address that is
    /// validly mapped but not yet backed by a physical page. This function
    /// allocates and installs the corresponding page into the page table on
    /// demand.
    ///
    /// The kernel may initialize the page from a file (if the mapping was
    /// file-backed) or zero-fill it (if anonymous). The newly loaded page
    /// must also be mapped with the correct permissions, as defined at the
    /// time of the original `mmap`.
    ///
    /// # Parameters
    /// - `page_table`: The page table of the faulting process.
    /// - `reason`: The [`PageFaultReason`] that describes the faulting reason.
    ///
    /// This must indicate a **demand paging fault**.
    ///
    /// # Returns
    /// - `Ok(())` if the page was successfully loaded and mapped.
    /// - `Err(KernelError)`: If the faulting address is invalid, out of bounds,
    ///   or if page allocation fails.
    pub fn do_lazy_load(
        &mut self,
        page_table: &mut PageTable,
        reason: &PageFaultReason,
    ) -> Result<(), KernelError> {
        todo!()
    }

    /// Handles a **page fault** by allocating a physical page and updating the
    /// page table.
    ///
    /// This function is called when a process accesses a lazily mapped page
    /// that has not been allocated yet. The function must:
    /// 1. Identify the faulting virtual address from [`PageFaultReason`].
    /// 2. Check if the address was previously recorded in `mmap` metadata.
    /// 3. Allocate a new physical page.
    /// 4. Update the page table with the new mapping.
    /// 5. Invalidate the TLB entry to ensure memory consistency.
    ///
    /// # Arguments
    /// - `page_table`: Mutable reference to the page table.
    /// - `reason`: The cause of the page fault, including the faulting address.
    ///
    /// If the faulting address was not mapped via `mmap`, the system should
    /// trigger a **segmentation fault**, resulting process exit.
    pub fn handle_page_fault(
        &mut self,
        page_table: &mut PageTable,
        reason: &PageFaultReason,
    ) -> Result<(), KernelError> {
        if reason.is_demand_paging_fault() {
            self.do_lazy_load(page_table, reason)
        } else if reason.is_cow_fault() {
            self.do_copy_on_write(page_table, reason)
        } else {
            Err(KernelError::InvalidAccess)
        }
    }
}
