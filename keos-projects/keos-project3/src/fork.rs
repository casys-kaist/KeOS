//! # `Fork` with Copy-On-Write optimization.
//
//! `fork` is a system call that creates a new process by duplicating the
//! calling process. The new child process is almost identical to the parent,
//! inheriting the same memory layout, open file descriptors, and register
//! state. The child receives a copy of the parent’s process state, including
//! [`FileStruct`] and [`MmStruct`]. Two processes can communicate via opened
//! `pipe`s after the forking. The only difference is the return value of
//! the syscall: the parent receives the child’s PID, while the child receives
//! 0.
//!
//! ### Copy-On-Write
//
//! In modern operating system, **fork** utilizes **copy-on-write (COW)**
//! optimization to efficiently share memory between parent and child. Instead
//! of copying all memory pages immediately, the parent and child initially
//! share all pages marked as read-only. If either process writes to one of
//! these shared pages, a page fault triggers the kernel to create a private
//! copy for that process.
//
//! Note that modern CPUs include a **Translation Lookaside Buffer (TLB)**, a
//! hardware cache that stores recent virtual-to-physical address translations.
//! This leads to case where even after you modify the permission of the
//! address, the change is **not immediately visible** to the CPU if the TLB
//! still holds a cached, now-stale mapping. Therefore, you must maintain the
//! consistency with the TLB. To maintain memory protection correctness:
//! - The kernel must **shut down** TLB for all pages made read-only by
//!   write-protection since they were previously writable.
//! - The kernel must **invalidate** a TLB entry after a new private page is
//!   installed , replacing a previously shared page.
//!
//! Without these TLB flushes, processes may continue using stale or incorrect
//! mappings, bypassing copy-on-write or causing data corruption.
//
//! In KeOS, copy-on-write works as follow:
//! 1. When a process invokes a **fork** system call, the kernel makes copy of
//!    [`FileStruct`].
//! 2. The kernel write-protected ptes by calling
//!    [`LazyPager::write_protect_ptes`] to make copy of [`MmStruct`]. This
//!    marks all writable pages as read-only when the child is created. This
//!    ensures any future writes will trigger a page fault.
//! 3. After write-protecting pages, the kernel **shuts down the TLB** entries
//!    for those pages to remove stale writable translations from the CPU's
//!    cache. This is done via [`tlb_shutdown`].
//! 4. Execute a new process for child with the copy of states.
//! 5. Resume the execution of both parent and child.
//!
//! After resuming the execution, process might confront a **page fault** from
//! the write-protect. The page fault handler determines whether the fault is
//! copy-on-write fault with [`PageFaultReason::is_cow_fault`] and handle it
//! with [`LazyPager::do_copy_on_write`]. This function finds the pte with
//! [`PageTable::walk_mut`], allocates and installs a new private copy of a
//! page. After mapping the new page, the kernel **invalidates the old TLB
//! entry** with the [`StaleTLBEntry::invalidate`].
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`LazyPager::write_protect_ptes`]
//! - [`PageFaultReason::is_cow_fault`]
//! - [`LazyPager::do_copy_on_write`]
//! - [`fork`]
//!
//! This ends the project 3.
//!
//! [`tlb_shutdown`]: keos::mm::page_table::tlb_shutdown

use crate::lazy_pager::{LazyPager, PageFaultReason};
#[cfg(doc)]
use keos::mm::page_table::StaleTLBEntry;
use keos::{KernelError, thread::ThreadBuilder};
use keos_project1::{file_struct::FileStruct, syscall::SyscallAbi};
use keos_project2::{mm_struct::MmStruct, page_table::PageTable};

impl LazyPager {
    /// Handles a copy-on-write (COW) page fault by creating a private copy of
    /// the faulted page.
    ///
    /// This method is invoked when a process attempts to write to a page that
    /// is currently shared and marked read-only as part of a copy-on-write
    /// mapping. It ensures that the faulting process receives its own
    /// writable copy of the page while preserving the original contents for
    /// other processes that may still share the original page.
    ///
    /// ### Steps:
    /// 1. Find write-protected page table entry with [`PageTable::walk_mut`].
    /// 2. Allocates a new page and copies the contents of the original page
    ///    into it.
    /// 3. Updates the page table to point to the new page with write
    ///    permissions.
    /// 4. Invalidates the TLB entry for the faulting address to ensure the CPU
    ///    reloads the mapping.
    ///
    /// ### Parameters
    /// - `page_table`: The faulting process’s page table.
    /// - `reason`: Information about the page fault, including the faulting
    ///   address and access type.
    pub fn do_copy_on_write(
        &mut self,
        page_table: &mut PageTable,
        reason: &PageFaultReason,
    ) -> Result<(), KernelError> {
        todo!()
    }

    /// Applies write-protection to all user-accessible pages in the memory
    /// layout.
    ///
    /// This method is called during `fork` to prepare the address space for
    /// copy-on-write semantics. It traverses the entire virtual memory
    /// layout, identifies writable mappings, and rewrites their page table
    /// entries (PTEs) as read-only. This allows parent and child
    /// processes to safely share physical memory until one performs a write, at
    /// which point a private copy is created.
    ///
    /// After modifying the page tables, stale entries in the **Translation
    /// Lookaside Buffer (TLB)** are invalidated to ensure that the CPU
    /// observes the new permissions by calling [`tlb_shutdown`].
    ///
    /// ### Parameters
    /// - `mm_struct`: The current process’s memory layout, including its
    ///   [`LazyPager`] state.
    ///
    /// ### Returns
    /// - A new [`MmStruct`] representing the forked child process, with updated
    ///   page table mappings.
    ///
    /// [`tlb_shutdown`]: keos::mm::page_table::tlb_shutdown
    pub fn write_protect_ptes(
        mm_struct: &mut MmStruct<LazyPager>,
    ) -> Result<MmStruct<LazyPager>, KernelError> {
        let MmStruct { page_table, pager } = mm_struct;
        let mut new_page_table = PageTable::new();
        todo!()
    }
}

impl PageFaultReason {
    /// Returns `true` if the fault is a **copy-on-write** violation.
    ///
    /// # Returns
    /// - `true` if this fault requires COW handling.
    /// - `false` otherwise.
    #[inline]
    pub fn is_cow_fault(&self) -> bool {
        todo!()
    }
}

/// Creates a new process by duplicating the current process using
/// copy-on-write.
///
/// `fork` is a system call that creates a child process that is
/// identical to the calling (parent) process. The child inherits the parent's
/// memory layout, file descriptors, and register state. After the fork, both
/// processes continue execution independently from the point of the call.
///
/// This implementation uses **copy-on-write (COW)** to avoid eagerly copying
/// the entire address space. Memory pages are initially shared between the
/// parent and child and marked as read-only. When either process attempts to
/// write to a shared page, a page fault occurs and
/// [`LazyPager::do_copy_on_write`] handles creating a private writable copy of
/// the page.
///
/// # Syscall API
/// ```c
/// int fork(void);
/// ```
///
/// ### Behavior
/// - The parent receives the child’s PID as the return value.
/// - The child receives `0` as the return value.
/// - On failure, the parent receives `Err(KernelError)` and no new process is
///   created.
///
/// ### Memory Management
/// - Invokes [`LazyPager::write_protect_ptes`] to mark shared pages as
///   read-only.
/// - Creates a new address space and page table for the child.
/// - Invalidates stale TLB entries to enforce new memory protection rules.
///
/// ### File Descriptors
/// - Duplicates the parent's file descriptor table.
/// - File objects are shared and reference-counted across parent and child,
///   consistent with the UNIX file model.
///
/// ### ABI and Register State
/// - Copies the parent’s ABI state into the child.
/// - Adjusts the child’s register state to reflect a return value of `0`.
///
/// ### Parameters
/// - `file_struct`: The parent’s file descriptor table to be duplicated.
/// - `mm_struct`: The parent’s memory layout (address space).
/// - `abi`: The parent’s syscall ABI and register snapshot.
/// - `create_task`: A closure for creating and spawning the new process.
///
/// ### Returns
/// - `Ok(pid)`: The parent receives the child process ID.
/// - `Err(KernelError)`: If the fork operation fails due to memory or resource
///   constraints.
pub fn fork(
    file_struct: &mut FileStruct,
    mm_struct: &mut MmStruct<LazyPager>,
    abi: &SyscallAbi,
    create_task: impl FnOnce(FileStruct, MmStruct<LazyPager>) -> ThreadBuilder,
) -> Result<usize, KernelError> {
    let file_struct = file_struct.clone();
    let mm_struct = LazyPager::write_protect_ptes(mm_struct)?;
    // TODO: Clone the register state and set the rax to be zero.
    let regs: keos::syscall::Registers = todo!();

    let handle = create_task(file_struct, mm_struct).spawn(move || regs.launch());
    Ok(handle.tid as usize)
}
