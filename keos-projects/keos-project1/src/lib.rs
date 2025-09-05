//! # Project 1: System Call
//!
//! In Project 1, both user applications and the kernel run at the same
//! privilege level. This is because the system does not yet support memory
//! isolation; thus, there is no hardware-enforced boundary between user and
//! kernel memory. In the next project, KeOS will introduce **privilege
//! separation**, leveraging hardware features to isolate user space from the
//! kernel, as is standard in modern operating systems.
//!
//! This project builds the **system call handling**, which is the groundwork
//! for secure and structured interaction between user applications and kernel
//! services in subsequent stages of the system.
//!
//! We are aware that you may not yet be familiar with the Rust language as well
//! as KeOS apis, so this project is intended to be much easier than later
//! projects.
//! Please consult the [`Rust Book`] if you are unfamiliar with Rust and be
//! familiar with Rust by solving exercises in [`Rustling`]. It is highly
//! recommend to tring those exercises if this is your first project in Rust.
//!
//! ## Getting Started
//!
//! To begin, navigate to the `keos-project1/grader` directory and run:
//!
//! ```bash
//! $ cargo run
//! ```
//!
//! Initially, KeOS will panic with a "not yet implemented" message, indicating
//! the missing components that you need to implement.
//!
//! ## Modifiable Files
//! In this project, you can modify the following two files:
//! - `syscall.rs`
//! - `file_struct.rs`
//!
//! When grading, other than theses two files are overwritten to the original
//! one.
//!
//! ## Project Outline
//! - [`System Call Infrastructure`]: Extract arguments from system call
//!   requests in a structured manner.
//! - [`File System Calls`]: Introduce system calls for file operations.
//!
//! [`System Call Infrastructure`]: syscall
//! [`File System Calls`]: file_struct
//! [`Rust Book`]: <https://doc.rust-lang.org/book/>
//! [`Rustling`]: <https://rustlings.rust-lang.org/>

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(unsafe_code)]
#![no_std]
#![no_main]

extern crate alloc;
#[allow(unused_imports)]
#[macro_use]
extern crate keos;

pub mod file_struct;
pub mod process;
pub mod syscall;

use keos::{KernelError, syscall::Registers, task::Task};
use syscall::SyscallAbi;

pub use process::Process;

/// Represents system call numbers used in project1.
///
/// Each variant corresponds to a specific system call that can be invoked
/// using the system call interface. The numeric values align with the
/// syscall table in the operating system.
#[repr(usize)]
pub enum SyscallNumber {
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
}

impl TryFrom<usize> for SyscallNumber {
    type Error = KernelError;
    fn try_from(no: usize) -> Result<SyscallNumber, Self::Error> {
        match no {
            1 => Ok(SyscallNumber::Open),
            2 => Ok(SyscallNumber::Read),
            3 => Ok(SyscallNumber::Write),
            4 => Ok(SyscallNumber::Seek),
            5 => Ok(SyscallNumber::Tell),
            6 => Ok(SyscallNumber::Close),
            7 => Ok(SyscallNumber::Pipe),
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
            SyscallNumber::Open => self.file_struct.open(&abi),
            SyscallNumber::Read => self.file_struct.read(&abi),
            SyscallNumber::Write => self.file_struct.write(&abi),
            SyscallNumber::Seek => self.file_struct.seek(&abi),
            SyscallNumber::Tell => self.file_struct.tell(&abi),
            SyscallNumber::Close => self.file_struct.close(&abi),
            SyscallNumber::Pipe => self.file_struct.pipe(&abi),
        });
        // Set the return value of the system call (success or error) back into the
        // registers.
        abi.set_return_value(return_val);
    }
}
