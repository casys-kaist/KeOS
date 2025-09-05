//! Project 5: File System
//!
//! Until project4, you have implemented the core features of modern OSes
//! including memory, multi-threading, and scheduling. In Project 5, you will
//! focus on implementing the file system, the abstraction of disk.
//!
//! This project will involve introducing **page cache** for caching a file
//! contents, and developing a **fast file system** with **journaling** to
//! ensure data integrity and recoverability after a system crash or failure.
//!
//! ## Getting Started
//!
//! To get started, navigate to the `keos-project5/grader` directory and run the
//! following command:
//!
//! ```bash
//! $ cargo run
//! ```
//!
//! ## Modifiable Files
//! In this project, you can modify the following files:
//! - `page_cache/mod.rs`
//! - `ffs/inode.rs`
//! - `ffs/journal.rs`
//! - `ffs/fs_objects.rs`
//! - `advanced_file_structs.rs`
//!
//! ## Project Outline
//! - [`Page Cache`]: Implement a caching mechanism to optimize access to
//!   frequently used files.
//! - [`Fast File System`]: Implement the simplfied version of fast file system
//!   with journaling.
//! - [`Advanced File System Call`]: Implement the advanced feature for the file
//!   system.
//!
//! [`Page Cache`]: page_cache
//! [`Fast File System`]: ffs
//! [`Advanced File System Call`]: advanced_file_structs

#![no_std]
#![no_main]
#![feature(slice_as_array, step_trait)]
#![deny(rustdoc::broken_intra_doc_links)]

extern crate alloc;
#[allow(unused_imports)]
#[macro_use]
extern crate keos;

macro_rules! const_assert {
    ($($tt:tt)*) => {
        const _: () = assert!($($tt)*);
    }
}

pub mod advanced_file_structs;
pub mod ffs;
pub mod lru;
pub mod page_cache;
pub mod process;

use core::ops::Range;

use advanced_file_structs::AdvancedFileStructs;
use alloc::{boxed::Box, collections::btree_set::BTreeSet};
use keos::{
    KernelError,
    addressing::{Pa, Va},
    sync::SpinLock,
    syscall::Registers,
    task::{PFErrorCode, Task},
    thread::with_current,
};
use keos_project1::syscall::SyscallAbi;
use keos_project3::{fork::fork, get_phys::get_phys};
pub use process::Thread;

#[doc(hidden)]
pub static ACCESS_CHECK_BYPASS_LIST: SpinLock<BTreeSet<Va>> = SpinLock::new(BTreeSet::new());

/// Represents system call numbers used in project5.
///
/// Each variant corresponds to a specific system call that can be invoked
/// using the system call interface. The numeric values align with the
/// syscall table in the operating system.
#[repr(usize)]
pub enum SyscallNumber {
    // == Pj 1 ==
    /// Terminates the calling thread.
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
    // == Pj 2 ==
    /// Map the memory.
    Mmap = 8,
    /// Unmap the memory.
    Munmap = 9,
    // == Pj 3 ==
    /// Fork the process.
    Fork = 10,
    // == Pj 4 ==
    /// Create a thread.
    ThreadCreate = 11,
    /// Join a Thread.
    ThreadJoin = 12,
    /// Terminates the process, by terminating all threads.
    ExitGroup = 13,
    // == Pj 5 ==
    /// Create a regular file.
    Create = 14,
    /// Make a directory.
    Mkdir = 15,
    /// Unlink a file.
    Unlink = 16,
    /// Change the current working directory.
    Chdir = 17,
    /// Read the contents of a directory.
    Readdir = 18,
    /// Stat a file.
    Stat = 19,
    /// Synchronize a file's in-memory state with disk.
    Fsync = 20,
    // == Grading Only ==
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
            11 => Ok(SyscallNumber::ThreadCreate),
            12 => Ok(SyscallNumber::ThreadJoin),
            13 => Ok(SyscallNumber::ExitGroup),
            14 => Ok(SyscallNumber::Create),
            15 => Ok(SyscallNumber::Mkdir),
            16 => Ok(SyscallNumber::Unlink),
            17 => Ok(SyscallNumber::Chdir),
            18 => Ok(SyscallNumber::Readdir),
            19 => Ok(SyscallNumber::Stat),
            20 => Ok(SyscallNumber::Fsync),
            0x81 => Ok(SyscallNumber::GetPhys),
            _ => Err(KernelError::NoSuchSyscall),
        }
    }
}

impl Task for Thread {
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
            SyscallNumber::Exit => self.exit_group(&abi),
            SyscallNumber::Open => self.with_file_struct_mut(|fs, abi| fs.open(abi), &abi),
            SyscallNumber::Read => self.with_file_struct_mut(|fs, abi| fs.read(abi), &abi),
            SyscallNumber::Write => self.with_file_struct_mut(|fs, abi| fs.write(abi), &abi),
            SyscallNumber::Seek => self.with_file_struct_mut(|fs, abi| fs.seek(abi), &abi),
            SyscallNumber::Tell => self.with_file_struct_mut(|fs, abi| fs.tell(abi), &abi),
            SyscallNumber::Close => self.with_file_struct_mut(|fs, abi| fs.close(abi), &abi),
            SyscallNumber::Pipe => self.with_file_struct_mut(|fs, abi| fs.pipe(abi), &abi),
            SyscallNumber::Mmap => {
                self.with_file_mm_struct_mut(|fs, mm, abi| mm.mmap(fs, abi), &abi)
            }
            SyscallNumber::Munmap => self.with_mm_struct_mut(|mm, abi| mm.munmap(abi), &abi),
            SyscallNumber::Fork => self.with_file_mm_struct_mut(
                |fs, mm, abi| {
                    fork(fs, mm, abi, |file_struct, mm_struct| {
                        with_current(|th| {
                            let builder = keos::thread::ThreadBuilder::new(&th.name);
                            let tid = builder.get_tid();
                            builder.attach_task(Box::new(Thread::from_fs_mm_struct(
                                file_struct,
                                mm_struct,
                                tid,
                            )))
                        })
                    })
                },
                &abi,
            ),
            SyscallNumber::ThreadCreate => self.thread_create(&abi),
            SyscallNumber::ThreadJoin => self.thread_join(&abi),
            SyscallNumber::ExitGroup => self.exit_group(&abi),
            SyscallNumber::Create => self.with_file_struct_mut(|fs, abi| fs.create(abi), &abi),
            SyscallNumber::Mkdir => self.with_file_struct_mut(|fs, abi| fs.mkdir(abi), &abi),
            SyscallNumber::Unlink => self.with_file_struct_mut(|fs, abi| fs.unlink(abi), &abi),
            SyscallNumber::Chdir => self.with_file_struct_mut(|fs, abi| fs.chdir(abi), &abi),
            SyscallNumber::Readdir => self.with_file_struct_mut(|fs, abi| fs.readdir(abi), &abi),
            SyscallNumber::Stat => self.with_file_struct_mut(|fs, abi| fs.stat(abi), &abi),
            SyscallNumber::Fsync => self.with_file_struct_mut(|fs, abi| fs.fsync(abi), &abi),
            SyscallNumber::GetPhys => {
                self.with_file_mm_struct_mut(|fs, mm, abi| get_phys(mm, fs, abi), &abi)
            }
        });
        // Set the return value of the system call (success or error) back into the
        // registers.
        abi.set_return_value(return_val);
    }

    #[inline]
    fn access_ok(&self, addr: Range<Va>, is_write: bool) -> bool {
        let guard = ACCESS_CHECK_BYPASS_LIST.lock();

        let result = if guard.contains(&addr.start) {
            true
        } else {
            self.0.access_ok(addr, is_write)
        };
        guard.unlock();
        result
    }

    #[inline]
    fn page_fault(&mut self, ec: PFErrorCode, cr2: Va) {
        self.0.page_fault(ec, cr2)
    }

    #[inline]
    fn with_page_table_pa(&self, f: &fn(Pa)) {
        self.0.with_page_table_pa(f)
    }
}
