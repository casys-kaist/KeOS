//! # Project 4: Synchronization and Multithreading
//!
//! In Project 4, you will expand KeOS's process abstraction to support
//! **multithreading**. You will implement basic **synchronization primitives**
//! such as **mutexes**, **condition variables**, and **semaphores** to
//! synchronize resources between threads.
//!
//! Synchronization primitives will be used to support correct multithreaded
//! behavior. For example, **thread join** functionality will be built using
//! semaphores, which themselves are implemented using a combination of mutexes
//! and condition variables.
//!
//! Finally, you will improve the scheduler by implementing a **round-robin**
//! scheduling algorithm. Unlike previous projects where the unit of scheduling
//! was the **entire process**, starting from this project, the unit of
//! scheduling becomes an individual **thread**.
//!
//! ## Getting Started
//!
//! To get started, navigate to the `keos-project4/grader` directory and run:
//!
//! ```bash
//! $ cargo run
//! ```
//!
//! ## Modifiable Files
//! In this project, you can modify the following five files:
//! - `sync/mutex.rs`
//! - `sync/condition_variable.rs`
//! - `sync/semaphore.rs`
//! - `process.rs`
//! - `round_robin.rs`
//!
//! ## Project Outline
//!
//! - [`Synchronization Primitives`]:
//!   - **Mutex**: Provide mutual exclusion.
//!   - **Condition Variable**: Enable waiting for conditions.
//!   - **Semaphore**: Implement higher-level synchronization using mutex and
//!     condition variables.
//!
//! - [`MultiThreading`]: Implement thread creation, termination, and and join
//!   mechanisms.
//!
//! - [`Round Robin Scheduler`]: Implement a round-robin scheduler that switches
//!   between threads fairly, using time slices.
//!
//!
//! [`Synchronization Primitives`]: sync
//! [`MultiThreading`]: process
//! [`Round Robin Scheduler`]: round_robin

#![no_std]
#![no_main]
#![feature(negative_impls)]
#![deny(rustdoc::broken_intra_doc_links)]

extern crate alloc;
#[allow(unused_imports)]
#[macro_use]
extern crate keos;

pub mod process;
pub mod round_robin;
pub mod sync;

use alloc::boxed::Box;
use core::ops::Range;
use keos::{
    KernelError,
    addressing::{Pa, Va},
    syscall::Registers,
    task::PFErrorCode,
    task::Task,
    thread::{ThreadBuilder, with_current},
};
use keos_project1::syscall::SyscallAbi;
use keos_project2::mm_struct::MmStruct;
use keos_project3::{fork::fork, get_phys::get_phys, lazy_pager::PageFaultReason};
pub use process::Thread;

/// Represents system call numbers used in project4.
///
/// Each variant corresponds to a specific system call that can be invoked
/// using the system call interface. The numeric values align with the
/// syscall table in the operating system.
#[repr(usize)]
pub enum SyscallNumber {
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
    /// Map the memory.
    Mmap = 8,
    /// Unmap the memory.
    Munmap = 9,
    /// Fork the process.
    Fork = 10,
    /// Create a thread.
    ThreadCreate = 11,
    /// Join a Thread.
    ThreadJoin = 12,
    /// Terminates the process, by terminating all threads.
    ExitGroup = 13,
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
            0x81 => Ok(SyscallNumber::GetPhys),
            _ => Err(KernelError::NoSuchSyscall),
        }
    }
}

impl Task for Thread {
    /// Handles a system call request from a user program.
    fn syscall(&mut self, regs: &mut Registers) {
        let abi = SyscallAbi::from_registers(regs); // Extract ABI from the registers.
        // Lookup the system call handler function based on the system call number.
        let return_val = SyscallNumber::try_from(abi.sysno).and_then(|no| match no {
            SyscallNumber::Exit => self.exit(&abi),
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
                            let builder = ThreadBuilder::new(&th.name);
                            let tid = builder.get_tid();
                            builder.attach_task(Box::new(Thread::from_file_mm_struct(
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
            SyscallNumber::GetPhys => {
                self.with_file_mm_struct_mut(|fs, mm, abi| get_phys(mm, fs, abi), &abi)
            }
        });
        // Set the return value of the system call (success or error) back into the
        // registers.
        abi.set_return_value(return_val);
    }

    /// Validates whether the given memory range is accessible for the process.
    fn access_ok(&self, addr: Range<Va>, is_write: bool) -> bool {
        self.with_mm_struct_mut(
            |mm_struct, (addr, is_write)| mm_struct.access_ok(addr, is_write),
            (addr, is_write),
        )
    }

    /// Handles a page fault.
    fn page_fault(&mut self, ec: PFErrorCode, cr2: Va) {
        if !self.with_mm_struct_mut(
            |mm_struct, (ec, cr2)| {
                let reason = PageFaultReason::new(ec, cr2);
                // Acquire a lock on the thread's memory state (`mm_state`) to ensure safe
                // access.

                // Delegate the fault handling to [`LazyPager::handle_page_fault`],
                // which will update the page table and allocate a physical page if necessary.
                let MmStruct { page_table, pager } = mm_struct;
                pager.handle_page_fault(page_table, &reason).is_ok()
            },
            (ec, cr2),
        ) {
            // If the fault is real fault, exit the process.
            let _ = self.exit_group(&SyscallAbi {
                sysno: SyscallNumber::ExitGroup as usize,
                arg1: -1isize as usize,
                arg2: 0,
                arg3: 0,
                arg4: 0,
                arg5: 0,
                arg6: 0,
                regs: &mut Registers::default(),
            });
            unreachable!()
        }
    }

    /// Runs a given closure with physical address of page table.
    fn with_page_table_pa(&self, f: &fn(Pa)) {
        f(self.page_table_pa)
    }
}
