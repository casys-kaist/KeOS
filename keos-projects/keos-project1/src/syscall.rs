//! # System call abi for x86_64.
//!
//! Operating systems provide an abstraction of hardware resources to user
//! programs, allowing them to interact with hardware without needing to
//! understand its complexities. The kernel is responsible for managing
//! resources such as memory, processes, and input/output devices, while
//! offering a simplified interface for user programs. System calls serve as a
//! crucial interface between user applications and the kernel, providing an
//! additional layer of abstraction that enables programs to request
//! services like file I/O and process management, without directly dealing with
//! the hardware.
//!
//! ## System Call Details
//!
//! The operating system deals with **software exceptions**, which occur
//! due to program execution errors such as a page fault or division by zero.
//! **Exceptions** are also the mechanism by which a user program requests
//! services from the operating system. These service requests are **system
//! calls**.
//!
//! In traditional x86 architecture, system calls were handled like any other
//! software exception through the `int` instruction. However, in x86-64
//! architecture, manufacturers introduced the `syscall` instruction, which
//! provides a fast and efficient way to invoke system calls.
//!
//! In modern systems, the `syscall` instruction is the most commonly used means
//! of invoking system calls. When a user program wants to make a system call,
//! it invokes the `syscall` instruction. The system call number and any
//! additional arguments are placed in registers before invoking `syscall`. Here
//! are the key details:
//!
//! 1. The **system call number** is passed in the `%rax` register.
//! 2. The **arguments** are passed in the registers `%rdi`, `%rsi`, `%rdx`,
//!    `%r10`, `%r8`, and `%r9`. Specifically:
//!    - `%rdi`: First argument
//!    - `%rsi`: Second argument
//!    - `%rdx`: Third argument
//!    - `%r10`: Fourth argument
//!    - `%r8`: Fifth argument
//!    - `%r9`: Sixth argument
//! 3. The **return value** is stored to the `%rax` register.
//!
//! ## Handling System Call in KeOS
//!
//! When the `syscall` instruction is invoked, the KeOS kernel serves the
//! requests by calling the [`Task::syscall`] method with the state of the CPU
//! captured in a structure called [`Registers`]. This structure contains the
//! state of all registers, which is accessible to the handler and allows for
//! the manipulation of register values (including the return value) within the
//! handler.
//!
//! On the beginning of the [`Task::syscall`] method, KeOS parses the arguments
//! of the system call by calling the [`SyscallAbi::from_registers`], which is
//! currently marked with `todo!()`. Your job is to extend this to retrieve
//! system call number and arguments according to the abi.
//!
//! After acquiring the [`SyscallAbi`] struct, KeOS dispatches the system call
//! handler based on the parsed system call number. This system call handler
//! handles the requests and returns the `Result<usize, KernelError>` to
//! indicate the result.
//!
//! ## Error Handling via `Result` Type
//!
//! Proper error handling is crucial for maintaining system stability and
//! ensuring that unexpected conditions do not lead to crashes or undefined
//! behavior. Errors incured by user **MUST NOT** stop the kernel.
//! When an error occurs, ensure that the system call returns an appropriate
//! error code rather than panicking or causing undefined behavior.
//! Providing meaningful error messages can also improve debugging and user
//! experience. Properly handling edge cases will help ensure a robust and
//! stable kernel implementation.
//!
//! To this end, KeOS combines the rust's `Result` type with [`KernelError`]
//! type to enumerate all possible errors that kernel can make.
//! For example, when implementing system call handlers, you can consider the
//! following error conditions:
//!
//! - [`KernelError::BadFileDescriptor`]: Attempting to operate on a file
//!   descriptor that is not open.
//! - [`KernelError::InvalidArgument`]: A process tries to perform an operation
//!   it lacks permission for (e.g., writing to a read-only file).
//! - [`KernelError::NoSuchEntry`]: A process attempts to open a non-existent
//!   file without specifying a creation flag.
//! - [`KernelError::BadAddress`]: A process attempts to access an unmapped or
//!   kernel memory.
//!
//! During the execution of a function, you might confront the errors. In such
//! cases, you should **never** handle the errors by panicking the kernel.
//! Instead, you propagates the error with `?` operator to the
//! [`Task::syscall`], which conveys the error code to the userspace. Consult
//! with [`RustBook`] for how to use `?` operator.
//!
//! ### Returning Value to User
//! The epilog of the [`Task::syscall`] captures both the result and error of
//! the function, interprets them in to the `usize`, and return the values into
//! the user space. You can update the user program's registers by directly
//! modify the fields of [`Registers`] captured in the [`SyscallAbi`].
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`SyscallAbi::from_registers`]
//! - [`SyscallAbi::set_return_value`]
//!
//! After implementing both methods, move on to the next [`Section`].
//!
//! [`Task::syscall`]: ../../keos/task/trait.Task.html#tymethod.syscall
//! [`Registers`]: ../../keos/syscall/struct.Registers.html
//! [`RustBook`]: <https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html>
//! [`Section`]: crate::file_struct

use keos::{KernelError, syscall::Registers};

/// A struct representing the system call ABI (Application Binary Interface).
///
/// This struct provides a way to access and manipulate the system call's
/// arguments and return values in the context of the system call handler. It
/// stores the system call number and up to six arguments that are passed to the
/// kernel during a system call, as well as a mutable reference to the CPU
/// registers ([`Registers`]) which hold the current state of the system call.
///
/// The [`SyscallAbi`] struct abstracts away the management of system call
/// parameters and return values, making it easier to implement and handle
/// system calls in the kernel.
///
/// [`Registers`]: ../../keos/syscall/struct.Registers.html
pub struct SyscallAbi<'a> {
    /// The system call number that identifies the requested system service.
    pub sysno: usize,
    /// First argument for the system call.
    pub arg1: usize,
    /// Second argument for the system call.
    pub arg2: usize,
    /// Third argument for the system call.
    pub arg3: usize,
    /// Fourth argument for the system call.
    pub arg4: usize,
    /// Fifth argument for the system call.
    pub arg5: usize,
    ///Sixth argument for the system call.
    pub arg6: usize,
    /// A mutable reference to the [`Registers`] structure, which holds the
    /// state of the CPU registers. It is used to manipulate the system
    /// call return value and access any state needed for the call.
    pub regs: &'a mut Registers,
}

impl<'a> SyscallAbi<'a> {
    /// Constructs a [`SyscallAbi`] instance from the provided registers.
    ///
    /// This function extracts the system call number and arguments from the
    /// [`Registers`] struct and initializes a new [`SyscallAbi`] struct.
    ///
    /// # Parameters
    ///
    /// - `regs`: A mutable reference to the [`Registers`] structure that
    ///   contains the current state of the CPU registers.
    ///
    /// # Returns
    ///
    /// Returns an instance of [`SyscallAbi`] populated with the system call
    /// number and arguments extracted from the provided registers.
    ///
    /// [`Registers`]: ../../keos/syscall/struct.Registers.html
    pub fn from_registers(regs: &'a mut Registers) -> Self {
        todo!()
    }

    /// Sets the return value for the system call.
    ///
    /// This function modifies the `%rax` register to indicate the result of the
    /// system call. If the system call was successful, it sets `%rax` to the
    /// returned value. If the system call encountered an error, it sets `%rax`
    /// to the corresponding error code with the [`KernelError::into_usize`]
    /// enum.
    ///
    /// # Parameters
    ///
    /// - `return_val`: A `Result` indicating either the success value
    ///   (`Ok(value)`) or the error type (`Err(KernelError)`).
    pub fn set_return_value(self, return_val: Result<usize, KernelError>) {
        // Set the return value in the registers based on the result.
        todo!()
    }
}
