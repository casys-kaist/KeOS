// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;
extern crate keos;
extern crate keos_project1;
#[macro_use]
extern crate grading;

mod syscall;

use alloc::boxed::Box;
use keos::SystemConfigurationBuilder;
pub use keos_project1::Process;

use crate::syscall::syscall_abi;

#[unsafe(no_mangle)]
pub unsafe fn main(_config_builder: SystemConfigurationBuilder) {
    if let Ok(fs) = simple_fs::FileSystem::load(1) {
        keos::info!("Filesystem: use `SimpleFS`.");
        keos::fs::FileSystem::register(fs)
    }

    keos::thread::ThreadBuilder::new("test-prehook")
        .attach_task(Box::new(syscall::SyscallAbiValidator::default()))
        .spawn(|| {
            keos::print!("Validate syscall abi...");
            syscall_abi();
            keos::TestDriver::<Process>::start([
                &syscall::open_normal,
                &syscall::open_invalid,
                &syscall::read_normal,
                &syscall::read_truncate,
                &syscall::read_error_bad_fd,
                &syscall::read_error_bad_mode,
                &syscall::read_error_bad_address,
                &syscall::write_normal,
                &syscall::write_sync,
                &syscall::write_persistence,
                &syscall::write_error_bad_fd,
                &syscall::write_error_bad_mode,
                &syscall::write_error_bad_address,
                &syscall::seek_begin,
                &syscall::seek_current,
                &syscall::seek_end,
                &syscall::seek_beyond_eof,
                &syscall::seek_error_stdio,
                &syscall::seek_error_bad_fd,
                &syscall::seek_error_bad_whence,
                &syscall::tell_basic,
                &syscall::tell_write,
                &syscall::tell_error_stdio,
                &syscall::tell_error_bad_fd,
                &syscall::stdio_normal,
                &syscall::stdio_partial,
                &syscall::stdout_normal,
                &syscall::stdout_empty,
                &syscall::stdout_invalid,
                &syscall::stderr_normal,
                &syscall::stderr_empty,
                &syscall::stderr_invalid,
                &syscall::close,
                &syscall::pipe_normal,
                &syscall::pipe_partial,
                &syscall::pipe_error_bad_direction,
                &syscall::pipe_error_bad_address,
            ]);
        });
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {}
