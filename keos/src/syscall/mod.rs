//! System call infrastructure.
use crate::thread::with_current;
pub use abyss::interrupt::Registers;
use abyss::x86_64::PrivilegeLevel;

pub mod uaccess;

#[doc(hidden)]
#[unsafe(no_mangle)]
pub extern "C" fn do_handle_syscall(frame: &mut Registers) {
    with_current(|th| match th.task.as_mut() {
        Some(task) => {
            task.syscall(frame);
        }
        _ => {
            panic!("Unexpected `syscall` instruction.")
        }
    });

    if frame.interrupt_stack_frame.cs.dpl() == PrivilegeLevel::Ring3 {
        crate::thread::__check_for_signal();
    }
}

/// Flags for system calls.
pub mod flags {
    /// The [`FileMode`] enum represents the access modes available when opening
    /// a file.
    ///
    /// This enum is used by user program to specify how a file is opened,
    /// determining which operations can be performed on the file. It
    /// defines three basic modes:
    /// - [`FileMode::Read`]: The file is opened for reading only.
    /// - [`FileMode::Write`]: The file is opened for writing only.
    /// - [`FileMode::ReadWrite`]: The file is opened for both reading and
    ///   writing.
    ///
    /// These modes are used to control how the file descriptor behaves when
    /// interacting with the file (e.g., reading, writing, or both).
    #[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone, Copy)]
    pub enum FileMode {
        /// Read-only access to the file.
        ///
        /// In this mode, the file can only be read from, and no changes can be
        /// made to the file's contents.
        Read = 0,

        /// Write-only access to the file.
        ///
        /// In this mode, the file can only be written to, and any existing
        /// content in the file is overwritten with new data.
        Write = 1,

        /// Both Read and Write access to the file.
        ///
        /// In this mode, the file can both be read and written, and does NOT
        /// removes existing content, but can be overwritten with new
        /// data.
        ReadWrite = 2,
    }
}
