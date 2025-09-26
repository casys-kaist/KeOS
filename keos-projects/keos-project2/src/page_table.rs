//! # Four-Level Page Table of x86_64
//!
//! One of the main roles of an operating system is resource abstraction. An
//! important resource in a computer system is memory. Each process operates in
//! its own memory space, which must be isolated from other processes. For
//! instance, your web browser should not have access to the memory used by your
//! music player. To ensure such isolation, hardware introduces a memory
//! protection mechanism that isolates the memory between processes.
//!
//! ## Virtual Memory
//!
//! The concept of virtual memory abstracts memory addresses from the underlying
//! physical storage device. Instead of directly accessing the physical memory,
//! addresses are translated through the Memory Management Unit (MMU). To
//! distinguish between these two types of addresses:
//! - A **virtual address** is used by programs before translation.
//! - A **physical address** refers to the actual location in memory after
//!   translation.
//!
//! A key distinction between virtual and physical addresses is that physical
//! addresses are unique and always refer to the same memory location. In
//! contrast, virtual addresses can be mapped to the same physical address or
//! different physical addresses at different times.
//!
//! ## Paging
//!
//! Paging is a memory management technique that divides both physical and
//! virtual memory into small, fixed-size chunks called **pages**. Typically, a
//! page is 4096 bytes in size. The mapping of physical and virtual memory is
//! managed via a **page table**, with each entry representing a page. The
//! active page table is typically managed through a special CPU register (e.g.,
//! `cr3` in x86_64).
//!
//! For every memory access, the CPU translates the virtual address to a
//! physical address using the page table. Since checking the page table for
//! every conversion would be inefficient, the CPU stores the results in a cache
//! called the **Translation Lookaside Buffer (TLB)**.
//!
//! The TLB is a CPU cache that stores recent translations of virtual memory
//! addresses to physical memory. The TLB is not updated automatically when the
//! page table is modified, so the kernel must explicitly invalidate the TLB
//! entries after a page table update. If you invalidate the entry, the kernel
//! may be work with a stale memory.
//!
//! ## Paging in x86_64
//!
//! x86_64 uses 4096-byte pages and employs a 4-level page table. Each table is
//! 4096 bytes in size, which is the same size as the page, and each entry in
//! the table is 8 bytes. This structure allows for a 48-bit physical address
//! space to be covered by the page table.
//!
//! The virtual address in x86_64 can be broken down into the following levels:
//! ```
//! 63          48 47            39 38            30 29            21 20         12 11         0
//! +-------------+----------------+----------------+----------------+-------------+------------+
//! | Sign Extend |    Page-Map    | Page-Directory | Page-directory |  Page-Table |    Page    |
//! |             | Level-4 Offset |    Pointer     |     Offset     |   Offset    |   Offset   |
//! +-------------+----------------+----------------+----------------+-------------+------------+
//!               |                |                |                |             |            |
//!               +------- 9 ------+------- 9 ------+------- 9 ------+----- 9 -----+---- 12 ----+
//!                                           Virtual Address
//! ```
//!
//! - The **sign extend** portion represents the higher bits, ensuring proper
//!   sign extension for the full address.
//! - The Page Map Level 4 (PM4) identifies the highest-level page directory.
//! - The subsequent levels (Page Directory, Page Table) map smaller chunks of
//!   virtual memory to physical memory.
//! - The **page offset** specifies the position within the 4096-byte page.
//!
//! A page must be **page-aligned**, meaning the virtual address must be
//! divisible by the page size (4096 bytes). The last 12 bits of the 64-bit
//! virtual address represent the page **offset**, while the upper bits are used
//! as indices for the page table.
//!
//! The page table also defines various attributes for each page, such as access
//! permissions (e.g., read/write/user). Note that the attributes from all
//! levels are **AND**ed together. This means attributes of the intermediate
//! level must contain all possible attributes.
//!
//! In x86_64, the `invlpg` instruction invalidates a specific TLB entry based
//! on the given virtual address. Note that the entire TLB is also flushed when
//! the `cr3` register is reloaded.
//!
//! ## Managing [`PageTable`] in KeOS
//! You need to implement x86_64's 4-level page table scheme. The core
//! abstraction about page table is [`PageTable`]. With this abstraction, you
//! will implement page table walking, mapping, and unmapping. In addition to
//! mapping and unmapping pages, the page table must be clear the entries when
//! it deallocates an associated memory. This traverses the entire 4-level page
//! table, unmapping each mapped virtual address and deallocating
//! the corresponding physical pages. After calling this method, all page table
//! levels—including the Page Directory Pointer Table (PDP), Page Directory
//! (PD), and Page Table (PT)—will be deallocated, leaving only the root Page
//! Map Level 4 (PML4).
//!
//! This [`PageTable`] represents a page table of a user process. Each process
//! has its own set of user pages, which reside below the kernel base address,
//! where pml4 index is lower than [`PageTableRoot::KBASE`]. The set of
//! kernel pages, however, is global and remains fixed in the virtual
//! address space, regardless of the running process or thread. These pages are
//! shared between multiple page tables, meaning that they **MUST NOT**
//! deallocated in any cases.
//!
//! KeOS already provides several abstractions to work with page table.
//! - The virtual and physical addresses: [`Pa`] and [`Va`].
//! - The Memory Permission: [`Permission`].
//! - Each table entry: [`Pml4e`], [`Pdpe`], [`Pde`], and [`Pte`].
//! - Flag of each table entry: [`Pml4eFlags`], [`PdpeFlags`], [`PdeFlags`], and
//!   [`PteFlags`].
//! - Invalidate a TLB entry: [`StaleTLBEntry::invalidate`].
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`PtIndices::from_va`]
//! - [`PageTable::do_map`]
//! - [`PageTable::unmap`]
//! - [`PageTable::walk`]
//! - [`PageTable::walk_mut`]
//! - [`PageTable::clear`]
//!
//! Make sure to implement the necessary functions for TLB
//! invalidation, and ensure the correct handling of memory protection and
//! access permissions for pages.
//!
//! By the end of this part, you will have built an essential component for
//! memory management, ensuring that processes can access their memory securely
//! and efficiently through the page table.
//! When you finish implementing all tasks, move on to the next [`section`].
//!
//! [`StaleTLBEntry`]: StaleTLBEntry
//! [`section`]: crate::mm_struct

use alloc::boxed::Box;
use core::ops::Deref;
use keos::{
    addressing::{Kva, Pa, Va},
    mm::{Page, page_table::*},
};

/// Represents page table indices for a given virtual address (VA).
///
/// In the x86_64 architecture, virtual addresses are translated to physical
/// addresses using a 4-level paging hierarchy:
/// - PML4 (Page Map Level 4)
/// - PDPT (Page Directory Pointer Table)
/// - PD (Page Directory)
/// - PT (Page Table)
///
/// This structure extracts the index values for each of these levels from a
/// virtual address. This struct provides a way to **decompose** a virtual
/// address into its respective page table indices.
pub struct PtIndices {
    /// The virtual address (VA) associated with this page table index
    /// breakdown.
    pub va: Va,

    /// Page Map Level 4 Index (PML4EI).
    pub pml4ei: usize,

    /// Page Directory Pointer table Index (PDPTEI).
    pub pdptei: usize,

    /// Page Directory Index (PDEI).
    pub pdei: usize,

    /// Page Table Index (PTEI).
    pub ptei: usize,
}

impl PtIndices {
    /// Extracts page table indices from a given virtual address ([`Va`]).
    ///
    /// This function takes a virtual address and calculates the corresponding
    /// PML4, PDPT, PD, and PT indices based on x86_64 paging structure.
    ///
    /// # Arguments
    /// - `va`: A virtual address ([`Va`]) to be broken down into page table
    ///   indices.
    ///
    /// # Returns
    /// - `Ok(Self)`: If `va` is page-aligned (i.e., lower 12 bits are zero).
    /// - `Err(PageTableMappingError::Unaligned)`: If `va` is not page-aligned.
    pub fn from_va(va: Va) -> Result<Self, PageTableMappingError> {
        if va.into_usize() & 0xFFF == 0 {
            Ok(Self {
                va,
                pml4ei: todo!(),
                pdptei: todo!(),
                pdei: todo!(),
                ptei: todo!(),
            })
        } else {
            Err(PageTableMappingError::Unaligned)
        }
    }
}

/// Page Table Structure for x86_64 Architecture.
///
/// This implements a 4-level page table structure for the x86_64 architecture.
/// It includes an inner structure ([`PageTableRoot`]) that stores the actual
/// entries for each of the 512 slots in the page table at each level. The
/// [`PageTable`] provides methods for mapping virtual addresses (VAs) to
/// physical addresses (PAs) with different permission levels, unmapping pages,
/// and walking the page table to find page table entries (PTEs)
/// for given virtual addresses. This is a fundamental part of managing virtual
/// memory in an operating system.
pub struct PageTable(pub Box<PageTableRoot>);

impl PageTable {
    /// Create an empty page table.
    ///
    /// This method initializes a new page table that allows to access every
    /// kernel address. The page table is represented as a
    /// `Box<PageTableRoot>`, which contains an array of 512 [`Pml4e`] entries.
    /// This structure is used in the 4-level paging system of x86_64
    /// architecture.
    ///
    /// # Returns
    /// A new [`PageTable`] instance with all entries initialized to zero.
    pub fn new() -> Self {
        Self(PageTableRoot::new_boxed_with_kernel_addr())
    }

    /// Get physical address of this page table.
    ///
    /// This method calculates the physical address (PA) corresponding to the
    /// current page table. It does this by converting the virtual address
    /// (VA) of the [`PageTable`] structure into a physical address.
    ///
    /// # Returns
    /// The physical address of the page table ([`Pa`]).
    pub fn pa(&self) -> Pa {
        Kva::new(self.0.as_ref().as_ptr() as usize)
            .unwrap()
            .into_pa()
    }

    /// Map a virtual address (`va`) to a physical page (`pg`) with the
    /// specified permissions (`perm`).
    ///
    /// This method updates the page table by mapping the provided virtual
    /// address to the given physical page, along with the requested
    /// permissions. It ensures that the virtual address is correctly mapped to
    /// the physical page, enabling proper memory access.
    ///
    /// # Arguments
    /// - `va`: The virtual address to map.
    /// - `pg`: The physical page to map to the virtual address.
    /// - `perm`: The permissions to apply to the mapping (e.g., read, write).
    ///
    /// # Returns
    /// A `Result` indicating success or failure of the mapping operation. If
    /// successful, it returns `Ok(())`. If there's an error (e.g., invalid
    /// virtual address or permissions), it returns an appropriate
    /// [`PageTableMappingError`].
    pub fn map(&mut self, va: Va, pg: Page, perm: Permission) -> Result<(), PageTableMappingError> {
        let pa = pg.into_raw();
        unsafe {
            self.do_map(va, pa, perm).inspect_err(|_| {
                Page::from_pa(pa);
            })
        }
    }

    /// Map a physical address (`pa`) to a virtual address (`va`) with the
    /// specified permissions (`perm`).
    ///
    /// # Safety
    /// This method is marked `unsafe` because it relies on the assumption
    /// that the physical address (`pa`) is valid.
    ///
    /// # Arguments
    /// - `va`: The virtual address to map.
    /// - `pa`: The physical address to map to the virtual address.
    /// - `perm`: The permissions to apply to the mapping (e.g., read, write).
    ///
    /// # Returns
    /// A `Result` indicating success or failure of the mapping operation. If
    /// successful, it returns `Ok(())`. If there's an error (e.g., invalid
    /// physical address or permissions), it returns an appropriate
    /// [`PageTableMappingError`].
    pub unsafe fn do_map(
        &mut self,
        va: Va,
        pa: Pa,
        perm: Permission,
    ) -> Result<(), PageTableMappingError> {
        let indices = PtIndices::from_va(va)?;
        // Hint: Use `Page::new()` to allocate tables.
        todo!()
    }

    /// Unmap the given virtual address (`va`) and return the physical page that
    /// was mapped to it.
    ///
    /// This method removes the mapping for the specified virtual address (`va`)
    /// and returns the physical page (`Page`) that was mapped to it, if
    /// such a mapping existed.
    ///
    /// # Arguments
    /// - `va`: The virtual address to unmap.
    ///
    /// # Returns
    /// A `Result` containing the physical page ([`Page`]) that was mapped to
    /// the given virtual address, or an error if the unmapping operation
    /// fails (e.g., the virtual address was not previously mapped).
    pub fn unmap(&mut self, va: Va) -> Result<Page, PageTableMappingError> {
        let indices = PtIndices::from_va(va)?;
        // Hint: Use `Page::from_pa()`.
        let pa = todo!();
        Ok(StaleTLBEntry::new(va, unsafe { Page::from_pa(pa) }).invalidate())
    }

    /// Walk through the page table to find reference to the corresponding page
    /// table entry (PTE) for the given virtual address (`va`).
    ///
    /// This method traverses the 4-level page table structure and returns a
    /// reference to the page table entry (Pte) for the specified virtual
    /// address, if such an entry exists.
    ///
    /// # Arguments
    /// - `va`: The virtual address to find the corresponding page table entry
    ///   for.
    ///
    /// # Returns
    /// A `Result` containing a reference to the page table entry (Pte)
    /// corresponding to the given virtual address, or an error if the entry
    /// does not exist (e.g., if the address is not mapped).
    pub fn walk(&self, va: Va) -> Result<&Pte, PageTableMappingError> {
        let indices = PtIndices::from_va(va)?;
        todo!()
    }

    /// Walk through the page table to find mutable reference for the
    /// corresponding page table entry (PTE) for the given virtual address
    /// (`va`).
    ///
    /// This method traverses the 4-level page table structure and returns a
    /// object to modify the page table entry (Walked) for the specified virtual
    /// address, if such an entry exists.
    ///
    /// # Arguments
    /// - `va`: The virtual address to find the corresponding page table entry
    ///   for.
    ///
    /// # Returns
    /// A `Result` containing a `Walked` referring to the page table entry (Pte)
    /// corresponding to the given virtual address, or an error if the entry
    /// does not exist (e.g., if the address is not mapped).
    pub fn walk_mut(&mut self, va: Va) -> Result<Walked<'_>, PageTableMappingError> {
        let indices = PtIndices::from_va(va)?;
        todo!()
    }

    /// Clears all entries from the page table and deallocates associated pages.
    ///
    /// This function traverses all levels of the page table, unmapping each
    /// mapped virtual address and deallocating the corresponding physical
    /// pages. After calling this method, the page table will be empty,
    /// including intermediate levels such as PDP, PD, and PT, except
    /// for the PML4 page itself, which remains allocated.
    ///
    /// This method is automatically called when a [`PageTable`] is dropped.
    ///
    /// # Behavior
    /// - Unmaps all virtual addresses currently mapped in the page table.
    /// - Frees all allocated pages, including intermediate-level page tables.
    /// - Leaves only the root page (PML4) intact.
    ///
    /// # Safety
    /// - Must only be called when no active process depends on the current
    ///   mappings.
    /// - Calling this on a live page table (e.g., the currently active one) may
    ///   result in undefined behavior.
    fn clear(&mut self) {
        // TODO: Clear the page table.
        // You must clear the mid-level table.
        // You don't need to care about pml4i indices larger than
        // [`PageTableRoot::KBASE`].
        todo!()
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PageTable {
    fn drop(&mut self) {
        assert_ne!(
            keos::intrinsics::read_cr3(),
            self.pa().into_usize(),
            "Trying to drop activated page table."
        );
        self.clear()
    }
}

/// A mutable reference to a page table entry (PTE) associated with a virtual
/// address.
///
/// `Walked` provides safe and convenient access for modifying an existing
/// mapping in the page table. It is typically the result of a successful page
/// table walk and includes both the virtual address and a mutable reference to
/// the corresponding page table entry.
///
/// This abstraction is useful for implementing operations such as clearing
/// mappings, updating physical page mappings, or changing permissions.
pub struct Walked<'a> {
    addr: Va,
    pte: &'a mut Pte,
}

impl Walked<'_> {
    /// Clears the current mapping by returning the physical page and a
    /// [`StaleTLBEntry`] for flushing the TLB.
    ///
    /// # Returns
    /// - `Some(StaleTLBEntry)` if the entry is mapped.
    /// - `None` if the entry is not valid.
    pub fn clear(&mut self) -> Option<StaleTLBEntry> {
        unsafe {
            self.pte
                .clear()
                .map(|pa| StaleTLBEntry::new(self.addr, Page::from_pa(pa)))
        }
    }

    /// Sets this page table entry to map to the given page with the specified
    /// flags.
    ///
    /// # Parameters
    /// - `page`: The physical page to be mapped.
    /// - `flags`: The desired access permissions and attributes for the
    ///   mapping.
    ///
    /// # Errors
    /// Returns `Err(PageTableMappingError)` if the physical address cannot be
    /// set (e.g., due to address alignment or capacity limits).
    pub fn set_page(&mut self, page: Page, flags: PteFlags) -> Result<(), PageTableMappingError> {
        if self.pte.flags().contains(PteFlags::P) {
            Err(PageTableMappingError::Duplicated)
        } else {
            unsafe {
                self.pte.set_pa(page.into_raw())?.set_flags(flags);
            }
            Ok(())
        }
    }
}

impl Deref for Walked<'_> {
    type Target = Pte;

    fn deref(&self) -> &Self::Target {
        self.pte
    }
}
