//! # Project 3: Advanced Memory Management
//!
//! In Project 3, you will expand KeOS's memory subsystem by implementing more
//! sophisticated techniques to manage memory efficiently. The focus will be on
//! two key features:
//!
//! - **Lazy Paging**: A strategy that delays allocation until a page is
//!   actually accessed, reducing memory usage and improving performance.
//!
//! - **Fork with Copy-On-Write**: An optimized process duplication mechanism
//!   where memory pages are initially shared between the parent and child
//!   processes, and only duplicated when a write occurs.
//!
//! Together, these mechanisms are essential for building a modern, efficient
//! operating system that handles memory-intensive workloads effectively.
//!
//! ## Getting Started
//!
//! To get started, navigate to the `keos-project3/grader` directory and run:
//!
//! ```bash
//! $ cargo run
//! ```
//!
//! ## Modifiable Files
//! In this project, you can modify the following two files:
//! - `lazy_pager.rs`
//! - `fork.rs`
//!
//! ## Project Outline
//!
//! - [`Lazy Paging`]: Implement demand paging, deferring memory allocation and
//!   actual page loading until the first access triggers a page fault.
//!
//! - [`Fork`]: Implement the `fork` system call using Copy-on-Write (COW),
//!   allowing processes to share memory pages efficiently until one attempts to
//!   modify them.
//!
//! [`Lazy Paging`]: lazy_pager
//! [`Fork`]: mod@crate::fork

#![no_std]
#![no_main]
#![deny(rustdoc::broken_intra_doc_links)]

extern crate alloc;
#[allow(unused_imports)]
#[macro_use]
extern crate keos;

pub mod fork;
pub mod get_phys;
pub mod lazy_pager;
pub mod process;

use alloc::boxed::Box;
use core::ops::Range;
use fork::fork;
use keos::{
    KernelError,
    addressing::{Pa, Va},
    syscall::Registers,
    task::PFErrorCode,
    task::Task,
    thread::{Current, ThreadBuilder, with_current},
};
use keos_project1::syscall::SyscallAbi;
use keos_project2::mm_struct::MmStruct;
#[cfg(doc)]
use lazy_pager::LazyPager;
use lazy_pager::PageFaultReason;
pub use process::Process;

/// Represents system call numbers used in project3.
///
/// Each variant corresponds to a specific system call that can be invoked
/// using the system call interface. The numeric values align with the
/// syscall table in the operating system.
#[repr(usize)]
#[derive(Debug)]
pub enum SyscallNumber {
    /// Terminates the calling process.
    Exit = 0,
    /// Opens a file and returns a file descriptor.
    Open = 1,
    /// Reads data from a file descriptor.
    Read = 2,
    /// Writes data to a file descriptor.
    Write = 3,
    /// Moves the file offset of an open file.
    Seek = 4,
    /// Retrieves the current file offset.
    Tell = 5,
    /// Closes an open file descriptor.
    Close = 6,
    /// Create an interprocess communication channel.
    Pipe = 7,
    /// Map the memory.
    Mmap = 8,
    /// Unmap the memory.
    Munmap = 9,
    /// Fork the process.
    Fork = 10,
    /// Get Physical Address of Page (for grading purposes only)
    GetPhys = 0x81,
}

impl TryFrom<usize> for SyscallNumber {
    type Error = KernelError;
    fn try_from(no: usize) -> Result<SyscallNumber, Self::Error> {
        match no {
            0 => Ok(SyscallNumber::Exit),
            1 => Ok(SyscallNumber::Open),
            2 => Ok(SyscallNumber::Read),
            3 => Ok(SyscallNumber::Write),
            4 => Ok(SyscallNumber::Seek),
            5 => Ok(SyscallNumber::Tell),
            6 => Ok(SyscallNumber::Close),
            7 => Ok(SyscallNumber::Pipe),
            8 => Ok(SyscallNumber::Mmap),
            9 => Ok(SyscallNumber::Munmap),
            10 => Ok(SyscallNumber::Fork),
            0x81 => Ok(SyscallNumber::GetPhys),
            _ => Err(KernelError::NoSuchSyscall),
        }
    }
}

impl Task for Process {
    /// Handles a system call request from a user program.
    ///
    /// This function is the entry point for system call handling. It retrieves
    /// the system call ABI from the provided [`Registers`] structure, which
    /// includes the system call number and its arguments. Based on the
    /// system call number (`sysno`), it looks up the appropriate handler
    /// function in a predefined list. If a handler is found, it is invoked
    /// with the ABI, otherwise, an error ([`KernelError::NoSuchSyscall`]) is
    /// returned.
    ///
    /// After the handler function processes the system call, the return value
    /// (either a success or error) is set back into the CPU registers via
    /// the `set_return_value` method. The return value is stored in the `%rax`
    /// register as per the x86-64 system call convention.
    ///
    /// # Parameters
    ///
    /// - `regs`: A mutable reference to the [`Registers`] struct, which
    ///   contains the current state of the CPU registers. This structure will
    ///   be used to retrieve the system call number and its arguments, and also
    ///   to set the return value.
    ///
    /// # Functionality
    ///
    /// The function processes the system call as follows:
    /// 1. Extracts the system call number and arguments using the
    ///    [`SyscallAbi::from_registers`].
    /// 2. Looks up the corresponding handler function from a predefined list of
    ///    system calls. The handler function is selected based on the system
    ///    call number (`sysno`).
    /// 3. If a handler is found, it is executed with the ABI as an argument. If
    ///    no handler is found, the function returns a
    ///    [`KernelError::NoSuchSyscall`] error.
    ///
    /// The result of the system call handler (either success or error) is then
    /// returned via the [`SyscallAbi::set_return_value`] method, which
    /// modifies the CPU registers accordingly.
    fn syscall(&mut self, regs: &mut Registers) {
        // ** YOU DON'T NEED TO CHANGE THIS FUNCTION **
        let abi = SyscallAbi::from_registers(regs); // Extract ABI from the registers.
        // Lookup the system call handler function based on the system call number.
        let return_val = SyscallNumber::try_from(abi.sysno).and_then(|no| match no {
            SyscallNumber::Exit => self.exit(&abi),
            SyscallNumber::Open => self.file_struct.open(&abi),
            SyscallNumber::Read => self.file_struct.read(&abi),
            SyscallNumber::Write => self.file_struct.write(&abi),
            SyscallNumber::Seek => self.file_struct.seek(&abi),
            SyscallNumber::Tell => self.file_struct.tell(&abi),
            SyscallNumber::Close => self.file_struct.close(&abi),
            SyscallNumber::Pipe => self.file_struct.pipe(&abi),
            SyscallNumber::Mmap => self.mm_struct.mmap(&mut self.file_struct, &abi),
            SyscallNumber::Munmap => self.mm_struct.munmap(&abi),
            SyscallNumber::Fork => fork(
                &mut self.file_struct,
                &mut self.mm_struct,
                &abi,
                |file_struct, mm_struct| {
                    with_current(|th| {
                        ThreadBuilder::new(&th.name).attach_task(Box::new(Process {
                            file_struct,
                            mm_struct,
                        }))
                    })
                },
            ),
            SyscallNumber::GetPhys => get_phys::get_phys(&self.mm_struct, &self.file_struct, &abi),
        });
        // Set the return value of the system call (success or error) back into the
        // registers.
        abi.set_return_value(return_val);
    }

    /// Validates whether the given memory range is accessible for the process.
    ///
    /// This function checks if a memory region is safe to read or write before
    /// performing a memory-related operation. It ensures that user-provided
    /// addresses do not access restricted memory regions, preventing
    /// potential security vulnerabilities or crashes.
    ///
    /// # Parameters
    /// - `addr`: A range of virtual addresses to be accessed.
    /// - `is_write`: `true` if the access involves writing to memory, `false`
    ///   for read-only access.
    ///
    /// # Returns
    /// - `true` if the memory range is valid and accessible.
    /// - `false` if access is denied due to invalid address range or
    ///   insufficient permissions.
    fn access_ok(&self, addr: Range<Va>, is_write: bool) -> bool {
        // Delegate the validation to the memory management system.
        self.mm_struct.access_ok(addr, is_write)
    }

    /// Handles a page fault by acquiring the memory state lock and calling
    /// [`LazyPager::handle_page_fault`].
    ///
    /// This function is typically invoked when a process accesses a memory
    /// region that has been lazily mapped but not yet backed by a physical
    /// page. It performs the following steps:
    /// 1. Locks the `mm_state` to ensure thread-safe access to the process's
    ///    memory state.
    /// 2. Calls [`LazyPager::handle_page_fault`], passing the locked `mm_state`
    ///    and fault reason.
    /// 3. The [`LazyPager::handle_page_fault`] function will allocate a
    ///    physical page and update the page table.
    fn page_fault(&mut self, ec: PFErrorCode, cr2: Va) {
        let reason = PageFaultReason::new(ec, cr2);

        // Delegate the fault handling to [`LazyPager::handle_page_fault`],
        // which will update the page table and allocate a physical page if necessary.
        let MmStruct { page_table, pager } = &mut self.mm_struct;
        if pager.handle_page_fault(page_table, &reason).is_err() {
            Current::exit(-1)
        }
    }

    fn with_page_table_pa(&self, f: &fn(Pa)) {
        f(self.mm_struct.page_table.pa())
    }
}
