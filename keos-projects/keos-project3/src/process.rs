//! The process model for project3.
//!
//! This file defines the process model of the project3.

use keos::{KernelError, thread::Current};
use keos_project1::{file_struct::FileStruct, syscall::SyscallAbi};
use keos_project2::mm_struct::MmStruct;

use crate::lazy_pager::LazyPager;

/// A process state of project 3, which contains file struct and mm struct.
pub struct Process {
    pub file_struct: FileStruct,
    pub mm_struct: MmStruct<LazyPager>,
}

impl Default for Process {
    fn default() -> Self {
        Self {
            file_struct: FileStruct::new(),
            mm_struct: MmStruct::new(),
        }
    }
}

impl Process {
    /// Create a process with given [`MmStruct`].
    pub fn from_mm_struct(mm_struct: MmStruct<LazyPager>) -> Self {
        Self {
            mm_struct,
            ..Default::default()
        }
    }

    /// Exit a process.
    ///
    /// This function terminates the calling thread by invoking `exit` on the
    /// current thread. The exit code is provided as the first argument
    /// (`arg1`) of the system call.
    ///
    /// # Syscall API
    /// ```c
    /// int exit(int status);
    /// ```
    /// - `status`: The thread's exit code.
    ///
    /// # Parameters
    /// - `abi`: A reference to `SyscallAbi`, which holds the arguments passed
    ///   to the system call.
    ///
    /// # Returns
    /// - Never returns.
    ///
    /// # Notes
    /// - This function does not return in normal execution, as it terminates
    ///   the thread.
    /// - If an error occurs, it returns a `KernelError`
    pub fn exit(&self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        Current::exit(abi.arg1 as i32)
    }
}
