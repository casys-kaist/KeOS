//! # Pager with Eager Paging Policy
//!
//! The [`EagerPager`] is a concrete implementation of the [`Pager`] trait used
//! in Project 2 to manage memory mapping behavior during `mmap` system calls.
//! As its name implies, it follows an **eager allocation strategy**: physical
//! pages are allocated and mapped into the page table **immediately** at the
//! time of `mmap`, regardless of whether they are subsequently accessed.
//!
//! This approach ensures that all virtual pages in the mapped region are backed
//! by initialized physical memory at the time of mapping. These pages are
//! typically zero-filled and mapped with the requested permissions (e.g., read,
//! write, execute). This simplifies the memory model and avoids page faults
//! after mapping, making [`EagerPager`] a useful baseline for implementing
//! memory management in early-stage systems like KeOS.
//!
//! The paging interface is defined by the [`Pager`] trait, which abstracts the
//! core functionality for virtual memory: allocation (`mmap`), deallocation
//! (`munmap`), access resolution (`get_user_page`), and permission checking
//! (`access_ok`).
//!
//! In later stages of KeOS (Project 3), you will implement demand paging using
//! `LazyPager`, which defers physical memory allocation until the first
//! access (e.g., page fault). That approach provides a more efficient
//! and realistic memory model used by modern operating systems.
//!
//! ## Memory Loading
//!
//! The eager pager supports both anonymous and file-backed memory mappings.
//! **Anonymous mappings** in eager paging are typically backed by
//! zero-initialized memory. On the other side, when mapping a **file-backed
//! page**, the [`RegularFile::mmap`] method must be used to register the
//! mapping. In Project 5, you will extend this method with the **page cache**,
//! a core mechanism that allows shared, read-consistent access to file data
//! across processes. Although [`EagerPager`] currently does not utilize the
//! page cache, the [`RegularFile`] interface and paging trait are designed to
//! accommodate it cleanly for future extensions.
//!
//! Note that, KeOS does not provide write-back behavior for the file-backed
//! pages.
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`EagerPager::mmap`]
//! - [`EagerPager::munmap`]
//! - [`EagerPager::get_user_page`]
//! - [`EagerPager::access_ok`]
//!
//! By implementing the eager pager, your kernel now have sufficient
//! functionalities to load user program in the next [`section`].
//!
//! [`section`]: crate::loader
use crate::{page_table::PageTable, pager::Pager};
use alloc::collections::btree_map::BTreeMap;
use keos::{
    KernelError,
    addressing::Va,
    fs::RegularFile,
    mm::{PageRef, page_table::Permission},
};

/// Represent a mapping of contiguous memory.
pub struct Mapping {
    /// Size of the area.
    mapping_size: usize,
    /// Permission of the area.
    perm: Permission,
}

/// [`EagerPager`] is a struct that implements the [`Pager`] trait.
/// It represents a pager strategy that is responsible for eager memory paging.
pub struct EagerPager {
    mappings: BTreeMap<Va, Mapping>,
}

impl Pager for EagerPager {
    /// Creates a new instance of [`EagerPager`].
    ///
    /// This constructor initializes an empty [`EagerPager`] struct.
    fn new() -> Self {
        Self {
            mappings: BTreeMap::new(),
        }
    }

    /// Memory map function (`mmap`) for eager paging.
    ///
    /// This function maps the given memory region into page table.
    /// Returns an address for the mapped area.
    fn mmap(
        &mut self,
        page_table: &mut PageTable,
        addr: Va,
        size: usize,
        prot: Permission,
        file: Option<&RegularFile>,
        offset: usize,
    ) -> Result<usize, KernelError> {
        todo!()
    }

    /// Memory unmap function (`munmap`) for eager paging.
    ///
    /// This function would unmap a previously mapped memory region, releasing
    /// any associated resources.
    fn munmap(&mut self, page_table: &mut PageTable, addr: Va) -> Result<usize, KernelError> {
        todo!()
    }

    /// Find a mapped page at the given virtual address.
    ///
    /// This function searches for a memory page mapped at `addr` and, if found,
    /// returns a tuple of [`PageRef`] to the page and its corresponding
    /// [`Permission`] flags.
    fn get_user_page(
        &mut self,
        page_table: &mut PageTable,
        addr: Va,
    ) -> Option<(PageRef<'_>, Permission)> {
        // HINT: use `PageRef::from_pa`
        todo!()
    }

    /// Checks whether access to the given virtual address is permitted.
    ///
    /// This function verifies that a virtual address `va` is part of a valid
    /// memory mapping and that the requested access type (read or write) is
    /// allowed by the page's protection flags.
    fn access_ok(&self, va: Va, is_write: bool) -> bool {
        todo!()
    }
}
