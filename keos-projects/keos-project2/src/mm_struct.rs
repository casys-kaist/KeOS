//! # Memory State of a process
//!
//! In the project 1, you implemented the file state management for a
//! process. An equally important state of a process is memory state.
//! This memory state involves managing virtual memory regions, tracking memory
//! allocations, and implementing memory deallocation when memory is no longer
//! needed. The operating system must track all active mappings per process,
//! enforce correct access permissions, and ensure proper cleanup when memory is
//! unmapped.
//!
//! ## Memory in KeOS
//!
//! The state of a process's memory is represented by the [`MmStruct`]
//! structure, similiar to the Linux kernel's `struct mm_struct`. Each process
//! maintains an [`MmStruct`] instance, which tracks the memory mapping state
//! for a process. This state plays a central role for serving a memory mapping
//! system calls: **mmap** and **munmap**.
//!
//! Memory mapping (`mmap`) allows processes to allocate, share, or access
//! memory in a flexible way. It is commonly used for:
//! - Allocating memory dynamically without relying on heap or stack growth.
//! - Mapping files into memory for fast access.
//! - Sharing memory between processes.
//!
//! The `mmap` system call establishes a mapping between a process's virtual
//! address space and either physical memory or a file. These memory mappings
//! are recorded within the **page table**, similar to how regular memory pages
//! are managed. However, unlike normal heap allocations, `mmap`-based
//! allocations allow more control over page protection, mapping policies, and
//! address alignment. When a process accesses an address within the mapped
//! region, a **page fault** occurs, triggering the operating system to allocate
//! and map the corresponding physical pages.
//!
//! The [`MmStruct`] contains the two key components:
//! - **Page Table**: Tracks mappings between virtual and physical addresses.
//! - **Pager**: Defines the policy for memory mapping and unmapping.
//!
//! ### Validating User Input
//!
//! One of the important aspects of managing memory safely is ensuring that
//! user input, such as memory addresses provided by system calls, is validated
//! correctly. **The kernel must never crash due to user input.**
//!
//! Many system calls, like `read` and `write`, rely on user-supplied memory
//! addresses—typically as buffer pointers. These addresses need to be carefully
//! validated before they can be used to ensure the integrity and stability of
//! the system.
//!
//! Without proper validation, several serious problems may arise:
//! - **Unauthorized access to kernel memory**: If the user is allowed to access
//!   kernel memory, this can lead to privilege escalation vulnerabilities,
//!   potentially giving the user more control than intended.
//! - **Dereferencing invalid pointers**: Accessing uninitialized or incorrectly
//!   mapped memory can result in segmentation faults or undefined behavior,
//!   leading to crashes or unexpected outcomes.
//! - **Memory corruption**: Improper handling of memory can result in
//!   corrupting the system's memory state, which can affect other processes,
//!   crash the kernel, or destabilize the entire system.
//!
//! [`MmStruct`] mitigate these risks with the [`MmStruct::access_ok`] method,
//! which ensures that the memory addresses provided by system calls are **valid
//! and safe** before being used. This validation mechanism will
//! prevent dangerous operations by checking if memory access is allowed, based
//! on the current process's memory layout and protection policies.
//!
//! By incorporating proper validation, [`MmStruct::access_ok`] ensures that
//! invalid memory accesses are handled **gracefully**, returning appropriate
//! errors rather than causing kernel panics or undefined behavior. This
//! validation mechanism plays a crucial role in maintaining system integrity
//! and protecting the kernel from potential vulnerabilities.

//! ### `Pager`
//!
//! The actual behavior of [`MmStruct`] lies in the [`Pager`] trait.
//! When a user program invokes those system calls, the [`MmStruct`] parses the
//! arguements from the [`SyscallAbi`] and forwarded them to an implementation
//! of the [`Pager`] traits, which provides the core interface for handling
//! memory mappings. Similarly, the core implementation to validate memory also
//! lies on the [`Pager::access_ok`].
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`MmStruct::mmap`]
//! - [`MmStruct::munmap`]
//! - [`MmStruct::access_ok`]
//!
//! After implementing them, move on to the next [`section`] to implement
//! paging policy, called `EagerPager`.
//!
//! [`section`]: crate::eager_pager

use crate::{page_table::PageTable, pager::Pager};
use core::ops::Range;
use keos::{
    KernelError,
    addressing::Va,
    fs::RegularFile,
    mm::{PageRef, page_table::Permission},
};
use keos_project1::{file_struct::FileStruct, syscall::SyscallAbi};

/// The [`MmStruct`] represents the memory state for a specific process,
/// corresponding to the Linux kernel's `struct mm_struct`.
///
/// This struct encapsulates the essential information required to manage
/// a process's virtual memory, including its page table and the pager
/// responsible for handling memory mapping operations (such as `mmap` and
/// `munmap`).
///
/// The [`MmStruct`] ensures that memory-related system calls and operations are
/// correctly applied within the process’s address space. It provides mechanisms
/// to allocate, map, and unmap memory pages, and serves as the interface
/// through which the OS kernel interacts with the user process’s memory layout.
///
/// # Memory State
///
/// The memory state includes the page table (referenced by `page_table_addr`)
/// that manages the virtual-to-physical address translations for the process,
/// and a pager (`pager`) that defines how memory-mapped files and dynamic
/// memory allocations are handled. Together, these components allow each
/// process to maintain its own isolated memory environment, supporting both
/// memory protection and efficient address space management.
///
/// Like its Linux counterpart, [`MmStruct`] plays a central role in memory
/// management, providing the kernel with per-process control over virtual
/// memory.
pub struct MmStruct<P: Pager> {
    /// The page table that maintains mappings between virtual and physical
    /// addresses.
    pub page_table: PageTable,

    /// The pager that handles memory allocation (`mmap`) and deallocation
    /// (`munmap`).
    pub pager: P,
}

impl<P: Pager> Default for MmStruct<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Pager> MmStruct<P> {
    /// Creates a new [`MmStruct`] with an empty page table and a new pager
    /// instance.
    ///
    /// # Returns
    /// - A new [`MmStruct`] instance initialized with a new [`PageTable`] and
    ///   `P::new()`.
    pub fn new() -> Self {
        Self {
            page_table: PageTable::new(),
            pager: P::new(), // Initialize the pager.
        }
    }
    // Check whether a given memory range is accessible by the process.
    ///
    /// This function ensures that system calls using memory addresses (such as
    /// `read`, `write`, etc.) operate only on **valid and accessible**
    /// memory regions.
    ///
    /// # Parameters
    /// - `addr`: A range of virtual addresses to be accessed.
    /// - `is_write`: `true` if the memory is being written to, `false` if it's
    ///   only being read.
    ///
    /// # Returns
    /// - `true` if the memory range is valid.
    /// - `false` if the memory range is invalid or inaccessible.
    pub fn access_ok(&self, addr: Range<Va>, is_write: bool) -> bool {
        todo!()
    }

    /// Wrapper function for the pager's `mmap` method. It delegates the actual
    /// memory mapping operation to the pager's `mmap` method.
    ///
    /// # Parameters
    /// - `fstate`: A mutable reference to the file state.
    /// - `abi`: The system call ABI, which contains the arguments for the
    ///   system call.
    ///
    /// # Returns
    /// - The result of the memory mapping operation, returned by the pager's
    ///   `mmap`.
    pub fn do_mmap(
        &mut self,
        addr: Va,
        size: usize,
        prot: Permission,
        file: Option<&RegularFile>,
        offset: usize,
    ) -> Result<usize, KernelError> {
        // Calls the real implementation in pager.
        let Self { page_table, pager } = self;
        pager.mmap(page_table, addr, size, prot, file, offset)
    }

    /// Maps a file into the process's virtual address space.
    ///
    /// This function implements the `mmap` system call, which maps either an
    /// anonymous mapping (fd = -1) or portion of a file into memory (fd >= 0).
    /// If the mapped region represent a file content, user programs can
    /// access the file contents as the part of the process’s memory.
    ///
    /// # Syscall API
    /// ```c
    /// void *mmap(void *addr, size_t length, int prot, int fd, off_t offset);
    /// ```
    /// - `addr`: Desired starting address of the mapping (must be page-aligned
    ///   and non-zero).
    /// - `length`: Number of bytes to map (must be non-zero).
    /// - `prot`: Desired memory protection flags.
    /// - `fd`: File descriptor of the file to be mapped.
    /// - `offset`: Offset in the file where mapping should begin.
    ///
    /// # Arguments
    ///
    /// * `fstate` - Mutable reference to the current file state.
    /// * `abi` - A reference to the system call arguments, including the file
    ///   descriptor, mapping length, protection flags, and file offset.
    ///
    /// # Behavior
    ///
    /// This function performs validation on the provided arguments before
    /// forwarding the request to the pager’s `mmap` method. The following
    /// conditions must be met:
    ///
    /// - `addr` must be non-zero and page-aligned.
    /// - `length` must be non-zero.
    /// - The file descriptor must refer to a regular file or -1 for anonymous
    ///   mapping.
    /// - The mapping must not overlap with any already mapped region, including
    ///   the user stack or any memory occupied by the program binary.
    ///
    /// Unlike Linux, KeOS does not support automatic address selection for
    /// `addr == NULL`, so `mmap` fails if `addr` is zero.
    ///
    /// If the length of the file is not a multiple of the page size, any excess
    /// bytes in the final page are zero-filled.
    ///
    /// # Returns
    ///
    /// Returns the starting virtual address of the mapped region on success, or
    /// a [`KernelError`] on failure due to invalid arguments or conflicts with
    /// existing memory mappings.
    pub fn mmap(
        &mut self,
        fstate: &mut FileStruct,
        abi: &SyscallAbi,
    ) -> Result<usize, KernelError> {
        self.do_mmap(todo!(), todo!(), todo!(), todo!(), todo!())
    }

    /// Unmaps a memory-mapped file region.
    ///
    /// This function implements the `munmap` system call, which removes a
    /// previously established memory mapping created by `mmap`. It releases
    /// the virtual memory associated with the mapping.
    ///
    /// # Syscall API
    /// ```c
    /// int munmap(void *addr);
    /// ```
    /// - `addr`: The starting virtual address of the mapping to unmap. This
    ///   must match the address returned by a previous call to `mmap` by the
    ///   same process and must not have been unmapped already.
    ///
    /// # Arguments
    ///
    /// * `fstate` - Mutable reference to the current file state.
    /// * `abi` - A reference to the system call arguments, including the
    ///   address to unmap.
    ///
    /// # Behavior
    ///
    /// - Unmaps the virtual memory region starting at `addr` that was
    ///   previously mapped via `mmap`.
    /// - Unmodified pages are simply discarded.
    /// - The virtual pages corresponding to the mapping are removed from the
    ///   process's address space.
    ///
    /// # Additional Notes
    ///
    /// - Calling `close` on a file descriptor or removing the file from the
    ///   filesystem does **not** unmap any of its active mappings.
    /// - To follow the Unix convention, mappings remain valid until they are
    ///   explicitly unmapped via `munmap`.
    ///
    /// # Returns
    ///
    /// Returns `Ok(0)` on success or a [`KernelError`] if the address is
    /// invalid or does not correspond to an active memory mapping.
    pub fn munmap(&mut self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        // Calls the pager's munmap method with placeholders for arguments.
        self.pager.munmap(&mut self.page_table, todo!())
    }

    /// Find a mapped page at the given virtual address and apply a function to
    /// it.
    ///
    /// This function searches for a memory page mapped at `addr` and, if found,
    /// applies the provided function `f` to it. The function `f` receives a
    /// [`PageRef`] to the page and its corresponding [`Permission`] flags.
    ///
    /// # Parameters
    /// - `addr`: The virtual address ([`Va`]) of the page to find.
    /// - `f`: A closure that takes a [`PageRef`] and its [`Permission`] flags,
    ///   returning a value of type `R`.
    ///
    /// # Returns
    /// - `Some(R)`: If the page is found, the function `f` is applied, and its
    ///   result is returned.
    /// - `None`: If no mapped page is found at `addr`.
    ///
    /// # Usage
    /// This method allows safe access to a mapped page. It is useful for
    /// performing read operations or permission checks on a page.
    ///
    /// # Safety
    /// - The rust's lifetime guarantees that the closure `f` never stores the
    ///   [`PageRef`] beyond its invocation.
    pub fn get_user_page_and<R>(
        &mut self,
        addr: Va,
        f: impl FnOnce(PageRef, Permission) -> R,
    ) -> Result<R, KernelError> {
        let Self { page_table, pager } = self;
        if let Some((pgref, perm)) = pager.get_user_page(page_table, addr) {
            Ok(f(pgref, perm))
        } else {
            Err(KernelError::BadAddress)
        }
    }
}
