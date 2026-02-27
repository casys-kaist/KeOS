//! # KeOS: KAIST Educational Operating System
//!
//! Welcome to the **KeOS** project!
//!
//! Operating systems (OS) form the backbone of modern computing, managing
//! hardware resources and providing a foundation for applications to run
//! efficiently. Understanding how an OS works is crucial for grasping the inner
//! workings of computer systems and software performance.
//!
//! **KeOS** is designed to offer a hands-on learning experience with core OS
//! components. Through this project, you will explore the fundamental
//! principles of OS design while building a minimal but functional operating
//! system from the ground up.
//!
//! We prioritize simplicity, focusing on the most essential aspects of OS
//! development. You won't have to worry about handling obscure edge cases or
//! hidden test cases. **The score you receive after running the grading scripts
//! is your final score.** Our goal is to make this project as straightforward
//! as possible. If you have suggestions on how we can further minimize
//! unnecessary complexity and focus on the core concepts, we encourage your
//! feedback.
//!
//! ## Project Structure
//!
//! The KeOS project is divided into five projects:
//!
//! 1. **[`System Call`]** – Learn how the OS interacts with user applications.
//! 2. **[`Memory Management`]** – Implement basic memory management and
//!    user-space execution.
//! 3. **[`Advanced Memory Management`]** – Expand the KeOS's memory management
//!    system with advanced features.
//! 4. **[`Process Management`]** – Implement the advanced process management,
//!    including round robin scheduler and sychronization primitives.
//! 5. **[`File System`]** – Develop a simple yet functional filesystem for data
//!    storage.
//!
//! Each project builds upon the previous ones, helping you progressively
//! develop a deeper understanding of OS design.
//!
//! ## Why Rust?
//!
//! We have chosen **Rust** for this project because of its **memory safety**,
//! **zero-cost abstractions**, and most importantly, its **concurrency model**.
//! These features make Rust an excellent choice for **operating system
//! development**.
//!
//! In traditional system programming languages, concurrency and memory bugs
//! such as data races, use-after-free errors, and null pointer dereferences are
//! common. Rust prevents these issues at compile time by enforcing strict
//! **ownership, borrowing, and lifetime rules**. This allows you to write
//! **safe and efficient concurrent code** without sacrificing performance, and
//! **reduces debugging time** for those bugs.
//!
//! By using Rust in **KeOS**, you will:
//! - **Develop safe and efficient concurrent programs** without the risk of
//!   data races.
//! - **Avoid common concurrency pitfalls** such as race conditions.
//!
//! ## Getting Started
//!
//! To set up your **KeOS** development environment, run the following commands:
//! ```bash
//! $ mkdir keos
//! $ cd keos
//! $ curl https://raw.githubusercontent.com/casys-kaist/KeOS/refs/heads/main/scripts/install-keos.sh | sh
//! ```
//!
//! We recommend using VS Code as the editor, along with `rust-analyzer` for
//! Rust support.
//!
//! - **DO NOT** make public forks of this project.
//! - The **KeOS license explicitly prohibits** public redistribution of this
//!   work.
//! - You **MUST NOT** share or distribute your work based on the provided
//!   template.
//!
//! Failure to comply may result in academic integrity violations.
//!
//! ## Grading
//!
//! During grading, we will **overwrite** all files **except those explicitly
//! whitelisted** for each project. Any modifications to non-whitelisted files
//! may result in a **zero score**, even if your implementation otherwise works
//! correctly.
//!
//! You can run the `cargo grade` command to check your current score locally.
//! This reported score will be treated as your **final grade**, as long as your
//! submission complies with the whitelist policy.
//!
//! Grading rubrics and the list of whitelisted files can be found in each
//! grader's `.grade-target` file.
//!
//! ### ⚠️ IMPORTANT NOTES on GRADING
//! - The **KeOS license explicitly prohibits** sharing or distributing your
//!   work based on the provided template.
//! - Please **DO NOT** make public forks of this project.
//! - **Cheating, plagiarism, or uploading your code online is strictly
//!   prohibited** and will result in **disqualification**.
//!
//! Failure to comply may result in academic integrity violations.
//!
//! For the grading, please refer to the following policy:
//! - Your submission **must pass all test cases without modifying the
//!   non-whitelisted code in each project**.
//! - Submissions that **fail to compile** will receive **0 points**.
//!
//!
//! ## Useful Materials
//! We strongly recommend reading the following materials to help you
//! successfully complete the KeOS project:
//! - [**Tips on Implementation**](tips)
//! - [**Debugging**](debugging)
//!
//! [`System Call`]: ../keos_project1
//! [`Memory Management`]: ../keos_project2
//! [`Advanced Memory Management`]: ../keos_project3
//! [`Process Management`]: ../keos_project4
//! [`File System`]: ../keos_project5

#![no_std]
#![allow(internal_features, static_mut_refs)]
#![feature(
    allocator_api,
    alloc_error_handler,
    alloc_layout_extra,
    lang_items,
    core_intrinsics,
    pointer_is_aligned_to,
    slice_as_array,
    step_trait
)]
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate abyss;
extern crate alloc;

// For documentations.
#[cfg(doc)]
pub mod debugging;
#[cfg(doc)]
pub mod tips;

pub mod channel;
pub mod fs;
#[doc(hidden)]
pub mod interrupt;
mod lang;
pub mod mm;
pub mod sync;
pub mod syscall;
pub mod task;
pub mod teletype;
pub mod thread;
pub mod util;

use abyss::spinlock;
pub use abyss::{
    x86_64::intrinsics,
    {MAX_CPU, addressing, debug, info, print, println, warning},
};
use alloc::{boxed::Box, collections::btree_set::BTreeSet, ffi::CString, vec::Vec};
use task::Task;
use thread::scheduler::{BOOT_DONE, Scheduler, scheduler};

/// Enum representing errors that can occur during a kernel operation.
///
/// This enum is used to categorize errors encountered by the kernel operation.
/// Each variant corresponds to a specific type of error that might
/// occur during the handling of a kernel operation. These errors can be
/// returned to the user program to indicate the nature of the failure.
#[derive(Debug, Eq, PartialEq)]
pub enum KernelError {
    /// Operation is not permitted. (EPERM)
    OperationNotPermitted,
    /// No such file or directory. (ENOENT)
    NoSuchEntry,
    /// IO Error. (EIO)
    IOError,
    /// Exec format error. (ENOEXEC)
    NoExec,
    /// BAD file descriptor. (EBADF)
    BadFileDescriptor,
    /// Out of memory. (ENOMEM)
    NoMemory,
    /// Permission denied. (EACCES)
    InvalidAccess,
    /// Bad address. (EFAULT)
    BadAddress,
    /// Device or resource busy. (EBUSY)
    Busy,
    /// File exists. (EEXIST)
    FileExist,
    /// Not a directory. (ENOTDIR)
    NotDirectory,
    /// Is a directory. (EISDIR)
    IsDirectory,
    /// Invalid arguement. (EINVAL)
    InvalidArgument,
    /// Too many open files. (EMFILE)
    TooManyOpenFile,
    /// No space left on device. (ENOSPC)
    NoSpace,
    /// Broken pipe. (EPIPE)
    BrokenPipe,
    /// File name too long. (ENAMETOOLONG)
    NameTooLong,
    /// Invalid system call number. (ENOSYS)
    NoSuchSyscall,
    /// Directory not empty (ENOTEMPTY)
    DirectoryNotEmpty,
    /// File system is corrupted. (EFSCORRUPTED)
    FilesystemCorrupted(&'static str),
    /// Operation is not supported. (ENOTSUPP)
    NotSupportedOperation,
}

impl KernelError {
    /// Converts the [`KernelError`] enum into a corresponding `usize` error
    /// code. The result is cast to `usize` for use as a return value in
    /// system calls.
    pub fn into_usize(self) -> usize {
        (match self {
            KernelError::OperationNotPermitted => -1isize,
            KernelError::NoSuchEntry => -2,
            KernelError::IOError => -5,
            KernelError::NoExec => -8,
            KernelError::BadFileDescriptor => -9,
            KernelError::NoMemory => -12,
            KernelError::InvalidAccess => -13,
            KernelError::BadAddress => -14,
            KernelError::Busy => -16,
            KernelError::FileExist => -17,
            KernelError::NotDirectory => -20,
            KernelError::IsDirectory => -21,
            KernelError::InvalidArgument => -22,
            KernelError::TooManyOpenFile => -24,
            KernelError::NoSpace => -28,
            KernelError::BrokenPipe => -32,
            KernelError::NameTooLong => -36,
            KernelError::NoSuchSyscall => -38,
            KernelError::DirectoryNotEmpty => -39,
            KernelError::FilesystemCorrupted(_) => -117,
            KernelError::NotSupportedOperation => -524,
        }) as usize
    }
}

/// The given `isize` does not indicate an [`KernelError`].
#[derive(Debug, Eq, PartialEq)]
pub struct TryFromError {
    e: isize,
}

impl TryFrom<isize> for KernelError {
    type Error = TryFromError;

    // Required method
    fn try_from(value: isize) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::OperationNotPermitted),
            -2 => Ok(Self::NoSuchEntry),
            -5 => Ok(Self::IOError),
            -8 => Ok(Self::NoExec),
            -9 => Ok(Self::BadFileDescriptor),
            -12 => Ok(Self::NoMemory),
            -13 => Ok(Self::InvalidAccess),
            -14 => Ok(Self::BadAddress),
            -16 => Ok(Self::Busy),
            -17 => Ok(Self::FileExist),
            -20 => Ok(Self::NotDirectory),
            -21 => Ok(Self::IsDirectory),
            -22 => Ok(Self::InvalidArgument),
            -24 => Ok(Self::TooManyOpenFile),
            -28 => Ok(Self::NoSpace),
            -32 => Ok(Self::BrokenPipe),
            -36 => Ok(Self::NameTooLong),
            -38 => Ok(Self::NoSuchSyscall),
            -39 => Ok(Self::DirectoryNotEmpty),
            -117 => Ok(Self::FilesystemCorrupted("")),
            -524 => Ok(Self::NotSupportedOperation),
            e => Err(TryFromError { e }),
        }
    }
}

#[doc(hidden)]
static mut KERNEL_CMDLINE: Option<CString> = None;

/// Panic depth level.
///
/// Used for determining double panic, and notifying drop handlers that panic is
/// in progress.
pub static PANIC_DEPTH: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// A builder for system configuration settings.
///
/// The [`SystemConfigurationBuilder`] struct provides an interface for
/// configuring various system-wide settings before initialization.
/// This builder is typically used during system setup to configure fundamental
/// aspects like scheduling policies, memory management, and other system-wide
/// parameters.
pub struct SystemConfigurationBuilder {
    _p: (), // Prevents external instantiation while allowing future extensions.
}

impl SystemConfigurationBuilder {
    /// Sets the system-wide scheduler.
    ///
    /// This function configures the default scheduler with a custom scheduler
    /// implementation. It is expected that the provided scheduler
    /// implements the [`Scheduler`] trait and has a `'static` lifetime,
    /// meaning it must outlive all references.
    pub fn set_scheduler(self, scheduler: impl Scheduler + 'static) {
        unsafe {
            thread::scheduler::set_scheduler(scheduler);
        }
    }
}

/// The entry of the KeOS for bootstrap processor.
#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe fn rust_main(core_id: usize, regions: abyss::boot::Regions, cmd: Option<&'static [u8]>) {
    info!(
        "\x1bc\n\
__  __     ___  ____  \n\
| |/ /___ / _ \\/ ___| \n\
| ' // _ \\ | | \\___ \\ \n\
| . \\  __/ |_| |___) |\n\
|_|\\_\\___|\\___/|____/ \n\
\n\
KAIST educational Operating System\n\
Copyright 2025 Computer Architecture and Systems Lab\n"
    );
    // Init memory.
    unsafe {
        info!("Memory: init memory.");
        crate::mm::init_mm(regions);
        if let Some(cmd) = cmd {
            KERNEL_CMDLINE = CString::from_vec_with_nul(cmd.into()).ok();
        }
    }
    info!("Devices: init pci devices.");
    unsafe {
        abyss::dev::pci::init();
    }
    // Load debug symbols
    info!("Panicking: Load debug symbols.");
    if !crate::lang::panicking::load_debug_infos() {
        warning!("Failed to read kernel image. Disabling stack backtrace.");
    }

    unsafe extern "Rust" {
        fn main(conf_builder: SystemConfigurationBuilder);
    }
    unsafe {
        main(SystemConfigurationBuilder { _p: () });
        abyss::boot::ap_init();
        // Kill the kernel low address.
        (abyss::addressing::Pa::new({
            unsafe extern "C" {
                static mut boot_pml4e: u64;
            }
            boot_pml4e as usize
        })
        .unwrap()
        .into_kva()
        .into_usize() as *mut mm::page_table::Pml4e)
            .as_mut()
            .unwrap()
            .set_flags(mm::page_table::Pml4eFlags::empty());
        println!(
            "Command line: {}",
            KERNEL_CMDLINE
                .as_ref()
                .and_then(|cmd| cmd.to_str().ok())
                .unwrap_or("")
        );
    }

    crate::interrupt::register(32, |_| scheduler().timer_tick());
    crate::interrupt::register(126, mm::tlb::handler);
    crate::interrupt::register(127, |_regs| { /* no-op */ });
    BOOT_DONE.store(true, core::sync::atomic::Ordering::SeqCst);
    // Now kernel is ready to serve task.
    crate::thread::scheduler::idle(core_id);
}

/// The entry of the KeOS for application processor.
#[unsafe(no_mangle)]
#[allow(clippy::missing_safety_doc)]
pub unsafe fn rust_ap_main(core_id: usize) {
    unsafe extern "Rust" {
        fn ap_main();
    }
    unsafe {
        ap_main();
    }
    crate::thread::scheduler::idle(core_id);
}

// Test utilities
#[doc(hidden)]
pub trait TestCase
where
    Self: Sync + Send,
{
    fn name(&'static self) -> &'static str;
    fn run(&'static self, task: Box<dyn Task>) -> bool;
}

impl<T> TestCase for T
where
    T: Fn() + Send + Sync + 'static,
{
    fn name(&'static self) -> &'static str {
        core::any::type_name::<T>()
    }
    fn run(&'static self, task: Box<dyn Task>) -> bool {
        print!("test {} ... ", core::any::type_name::<T>());
        if crate::thread::ThreadBuilder::new(core::any::type_name::<T>())
            .attach_task(task)
            .spawn(self)
            .join()
            == 0
        {
            println!("ok");
            true
        } else {
            println!("FAILED");
            false
        }
    }
}

/// A driver for running tests.
pub struct TestDriver<T: Task + Default + 'static> {
    _t: core::marker::PhantomData<T>,
}

impl<T: Task + Default + 'static> TestDriver<T> {
    /// Run the given tests.
    pub fn start<const TC: usize>(tests: [&'static dyn TestCase; TC]) {
        crate::thread::ThreadBuilder::new("test_main").spawn(move || {
            let tests = match unsafe { KERNEL_CMDLINE.as_ref() } {
                Some(cmd) if cmd.to_str() != Ok("") => {
                    let filter = cmd
                        .to_str()
                        .expect("Failed to parse cmd")
                        .split(" ")
                        .collect::<BTreeSet<_>>();
                    tests
                        .iter()
                        .filter(|test| {
                            let name = test.name();
                            let r = name.split("::").next().map(|n| n.len() + 2).unwrap_or(0);
                            filter.contains(&name[r..])
                        })
                        .collect::<Vec<_>>()
                }
                _ => tests.iter().collect::<Vec<_>>(),
            };
            let (total, mut succ) = (tests.len(), 0);
            println!(
                "Running {} test{}",
                total,
                if total == 1 { "" } else { "s" }
            );

            for test in tests {
                if test.run(Box::new(T::default())) {
                    succ += 1;
                }
            }
            println!(
                "test result: {}. {} passed; {} failed",
                if total == succ { "ok" } else { "FAILED" },
                succ,
                total - succ
            );

            unsafe {
                abyss::x86_64::power_control::power_off();
            }
        });
    }
}
