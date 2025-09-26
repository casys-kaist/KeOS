//! Pager, a trait for paging policy
use crate::page_table::PageTable;
use keos::{
    KernelError,
    addressing::Va,
    fs::RegularFile,
    mm::{PageRef, page_table::Permission},
};

/// The [`Pager`] trait defines the interface for memory paging operations,
/// including memory mapping (`mmap`), unmapping (`munmap`), and resolving
/// page through [`get_user_page`]. It serves as the core abstraction
/// for implementing paging policies in the operating system.
///
/// This trait allows the OS to support multiple paging strategies by swapping
/// in different implementations. Each [`Pager`] manages how virtual memory
/// pages are backed, populated, and protected during execution.
///
/// In **Project 2**, you will use [`EagerPager`], which eagerly allocates all
/// required pages at the time of `mmap`.
///
/// In **Project 3**, you will implement [`LazyPager`], which delays page
/// allocation until the page is accessed (i.e., implements demand paging).
///
/// [`LazyPager`]: ../../keos_project3/lazy_pager/struct.LazyPager.html
/// [`EagerPager`]: ../../keos_project2/eager_pager/struct.EagerPager.html
/// [`get_user_page`]: Pager::get_user_page
pub trait Pager {
    /// Creates a new instance of the pager.
    ///
    /// This method initializes the internal state needed for the pager's
    /// memory management operations.
    fn new() -> Self;

    /// Maps a virtual memory region into the process’s address space.
    ///
    /// This function is responsible for allocating and mapping virtual memory
    /// pages. If a file is provided, it may also initialize the memory
    /// contents from that file (starting at `offset`). The mapped memory
    /// should be configured with the specified access `prot`.
    ///
    /// # Parameters
    /// - `page_table`: The current page table of the process.
    /// - `addr`: The starting virtual address of the mapping. Must be
    ///   page-aligned.
    /// - `size`: The size of the region to map in bytes. Must be greater than
    ///   zero.
    /// - `prot`: Memory protection flags (e.g., read, write, execute).
    /// - `file`: Optional file backing for the mapping.
    /// - `offset`: Offset into the file where the mapping begins.
    ///
    /// # Returns
    /// - `Ok(n)`: Number of bytes successfully mapped.
    /// - `Err([KernelError])`: If the operation fails, e.g., due to invalid
    ///   arguments, overlapping mappings, zero-length file, or a
    ///   non-page-aligned `addr`.
    fn mmap(
        &mut self,
        page_table: &mut PageTable,
        addr: Va,
        size: usize,
        prot: Permission,
        file: Option<&RegularFile>,
        offset: usize,
    ) -> Result<usize, KernelError>
    where
        Self: Sized;

    /// Unmaps a previously mapped memory region.
    ///
    /// This function removes memory mappings corresponding to `addr`.
    ///
    /// # Parameters
    /// - `page_table`: The process’s page table to modify.
    /// - `addr`: Starting virtual address of the region to unmap.
    ///
    /// # Returns
    /// - `Ok(n)`: Number of bytes successfully unmapped.
    /// - `Err([KernelError])`: On invalid addresses, unmapped regions, or other
    ///   errors.
    fn munmap(&mut self, page_table: &mut PageTable, addr: Va) -> Result<usize, KernelError>
    where
        Self: Sized;

    /// Resolves a virtual address to a page reference.
    ///
    /// This function checks whether a mapping exists for the virtual address
    /// `addr` and, if so, returns the in-memory page and its access
    /// permissions.
    ///
    /// # Parameters
    /// - `page_table`: The current process's page table.
    /// - `addr`: The virtual address to resolve.
    ///
    /// # Returns
    /// - `Some((PageRef, Permission))`: If the page exists and is accessible.
    /// - `None`: If no page is mapped at `addr`.
    fn get_user_page(
        &mut self,
        page_table: &mut PageTable,
        addr: Va,
    ) -> Option<(PageRef<'_>, Permission)>
    where
        Self: Sized;

    /// Checks whether access to the given virtual address is permitted.
    ///
    /// This function verifies that a virtual address `va` is part of a valid
    /// memory mapping and that the requested access type (read or write) is
    /// allowed by the page's protection flags.
    ///
    /// It is used by system calls and the kernel to ensure that user-level
    /// memory accesses (e.g., writing to buffers or reading strings) are safe
    /// and legal. This check prevents unauthorized access, such as writing to
    /// read-only pages or accessing unmapped memory.
    ///
    /// Unlike `get_user_page`, this function does **not** resolve or load the
    /// page into memory. It only checks that the memory mapping is valid for
    /// the given access type.
    ///
    /// # Parameters
    /// - `va`: The virtual address to check.
    /// - `is_write`: Indicates whether the access is for writing (`true`) or
    ///   reading (`false`).
    /// # Returns
    /// - `true`: If the page exists and is accessible with the permission.
    /// - `false`: If no page is mapped at `addr` or permission mismatches.
    fn access_ok(&self, va: Va, is_write: bool) -> bool;
}
