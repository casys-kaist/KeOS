// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc)]
#![feature(iter_array_chunks)]

extern crate alloc;
extern crate keos;
extern crate keos_project1;
extern crate keos_project2;
extern crate keos_project3;
extern crate keos_project4;
#[macro_use]
extern crate grading;

mod round_robin;
mod sync;
mod userprog;

use keos::SystemConfigurationBuilder;
pub use keos_project4::Thread;
use keos_project4::round_robin::RoundRobin;

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe fn main(config_builder: SystemConfigurationBuilder) {
    if let Ok(fs) = simple_fs::FileSystem::load(1) {
        keos::info!("Filesystem: use `SimpleFS`.");
        keos::fs::FileSystem::register(fs)
    }
    config_builder.set_scheduler(RoundRobin::new());
    keos::TestDriver::<Thread>::start([
        // Round robin Scheduler.
        &round_robin::functionality,
        &round_robin::balance,
        &round_robin::balance2,
        &round_robin::affinity,
        // Sync
        &sync::mutex::smoke,
        &sync::mutex::parking,
        &sync::mutex::smoke_many,
        &sync::condition_variable::bounded_buffer_1,
        &sync::condition_variable::bounded_buffer_2,
        &sync::semaphore::sema_0,
        &sync::semaphore::sema_1,
        &sync::semaphore::sema_2,
        &sync::semaphore::exec_order,
        &sync::semaphore::n_permits,
        // Loader.
        &userprog::arg_parse,
        &userprog::sys_open,
        &userprog::sys_read,
        &userprog::sys_read_error,
        &userprog::sys_write,
        &userprog::sys_write_error,
        &userprog::sys_seek,
        &userprog::sys_seek_error,
        &userprog::sys_tell,
        &userprog::sys_tell_error,
        &userprog::sys_stdio_1,
        &userprog::sys_stdio_2,
        &userprog::sys_stdout,
        &userprog::sys_stderr,
        &userprog::sys_pipe,
        &userprog::mm_mmap,
        &userprog::mm_mmap_error_protection,
        &userprog::mm_mmap_error_protection_exec,
        &userprog::mm_munmap,
        &userprog::mm_munmap_error,
        &userprog::bad_addr_1,
        &userprog::bad_code_write,
        // User thread.
        &userprog::thread_create,
        &userprog::thread_join_err,
        &userprog::thread_join_chain,
        &userprog::thread_join_complex,
        &userprog::thread_mm_shared,
    ]);
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {}
