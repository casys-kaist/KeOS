//! # Project 2: Memory Management
//!
//! In Project 2, you will expand your operating system with memory management
//! system, forming the isolation boundary between user applications and the
//! kenrel. This project builds upon the concepts from Project 1 and introduces
//! key topics such as **page tables**, **memory state management**, and **user
//! program loading**. By the end of this project, your kernel will be capable
//! of running multiple user programs in an isolated, virtual memory
//! environment.
//!
//! ## Getting Started
//!
//! To begin, navigate to the `keos-project2/grader` directory and run:
//!
//! ```bash
//! $ cargo run
//! ```
//!
//! ## Modifiable Files
//! In this project, you can modify the following six files:
//! - `page_table.rs`
//! - `mm_struct.rs`
//! - `eager_pager.rs`
//! - `loader/mod.rs`
//! - `loader/elf.rs`
//! - `loader/stack_builder.rs`
//!
//! ## Project Outline
//!
//! This project consists of three major components:
//!
//! - [`Page Table`]: Implement an x86_64 page table mechanism to manage virtual
//!   memory and perform address translation for user processes.
//!
//! - [`Memory State`]: Develop memory state management and implement system
//!   calls for dynamic memory allocation and deallocation.
//!
//! - [`User Program Loader`]: Build a loader that loads ELF executables into
//!   memory, sets up the stack, and transitions into user mode execution.
//!
//! Successfully completing this project will enable KeOS to run simple C
//! programs, setting the stage for more complex user-space features in future
//! projects.
//!
//! ## Debugging a User Process
//!
//! Because KeOS's internal backtrace only support for kernel's virtual
//! addresses, user program's instruction address may be shown as unknown.
//! For such cases, you may utilize `addr2line` utility to see which user mode
//! codes are responsible for the error.
//! For example, if you get the following error message:
//!
//! ```plaintext
//! User process ABORT at sys_mmap_err.c:14 in main(): assertion `write(1, NULL, 0x1000) < 0' failed.
//! Call stack: 0x402a83 0x401100 0x402a02
//! The `addr2line' program can make call stacks useful.
//! Read "Debugging a User Process" chapter in the
//! KeOS documentation for more information.
//! ```
//!
//! You translate the address to respective source code's line by passing the
//! program's name and address to `addr2line` utility:
//!
//! ```bash
//! $ addr2line -e sys_mmap_err 0x402a83
//! ../../kelibc/debug.c:29
//! $ addr2line -e sys_mmap_err 0x401100
//! userprog/sys_mmap_err.c:16
//! ```
//!
//!
//! [`Page Table`]: page_table
//! [`Memory State`]: mm_struct
//! [`User Program Loader`]: loader

#![no_std]
#![no_main]
#![deny(rustdoc::broken_intra_doc_links)]

extern crate alloc;
#[allow(unused_imports)]
#[macro_use]
extern crate keos;

pub mod eager_pager;
pub mod loader;
pub mod mm_struct;
pub mod page_table;
pub mod pager;
pub mod process;

use core::ops::Range;
use keos::{
    KernelError,
    addressing::{Pa, Va},
    syscall::Registers,
    task::Task,
};
use keos_project1::syscall::SyscallAbi;

pub use process::Process;

/// Represents system call numbers used in project2
///
/// Each variant corresponds to a specific system call that can be invoked
/// using the system call interface. The numeric values align with the
/// syscall table in the operating system.
#[repr(usize)]
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

    fn with_page_table_pa(&self, f: &fn(Pa)) {
        f(self.mm_struct.page_table.pa())
    }
}
