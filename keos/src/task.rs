//! Task trait for interact with user process.

use crate::thread::kill_current_thread;
pub use abyss::x86_64::interrupt::PFErrorCode;
use abyss::{
    addressing::{Pa, Va},
    interrupt::Registers,
};
use core::ops::Range;

/// Represents a **task** executed by a thread.
///
/// This trait defines core functionalities required for handling event
/// triggered by user process, such as **system calls**, **page faults**.
pub trait Task {
    /// Handles a **system call** triggered by the user program.
    ///
    /// - The `registers` parameter contains the state of the CPU registers at
    ///   the time of the system call.
    /// - Implementations of this function should parse the system call
    ///   arguments, execute the corresponding operation, and store the result
    ///   back in `registers`.
    fn syscall(&mut self, registers: &mut Registers);

    /// Handles a **page fault** that occurs when accessing an unmapped memory
    /// page.
    ///
    /// - The `ec` parameter provides information about the cause of the page
    ///   fault.
    fn page_fault(&mut self, ec: PFErrorCode, cr2: Va) {
        if (ec & PFErrorCode::USER) == PFErrorCode::USER {
            kill_current_thread();
        } else {
            panic!(
                "Unexpected page fault in Kernel at {:?} because of {:?}",
                cr2, ec
            );
        }
    }

    /// Validates a given **memory address range** before use.
    ///
    /// - `addr`: The range of virtual addresses being accessed.
    /// - `is_write`: Indicates whether the memory is being **read** (`false`)
    ///   or **written to** (`true`).
    /// - Returns `true` if the memory range is valid, or an appropriate
    ///   `KernelError` otherwise.
    #[allow(unused_variables)]
    fn access_ok(&self, addr: Range<Va>, is_write: bool) -> bool {
        // Currently, check only addr is null pointer.
        addr.start.into_usize() != 0
    }

    /// Run a closure with physical address of the page table.
    fn with_page_table_pa(&self, _f: &fn(Pa)) {}
}

impl Task for () {
    fn syscall(&mut self, _registers: &mut Registers) {
        unreachable!()
    }
}
