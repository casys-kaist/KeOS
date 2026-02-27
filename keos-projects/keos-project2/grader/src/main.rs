// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;
extern crate keos;
extern crate keos_project1;
extern crate keos_project2;
#[macro_use]
extern crate grading;

mod mm_struct;
mod page_table;
mod userprog;

use keos::SystemConfigurationBuilder;
pub use keos_project2::Process;

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe fn main(_config_builder: SystemConfigurationBuilder) {
    if let Ok(fs) = simple_fs::FileSystem::load(1) {
        keos::info!("Filesystem: use `SimpleFS`.");
        keos::fs::FileSystem::register(fs)
    }
    keos::TestDriver::<Process>::start([
        // Page table.
        &page_table::simple,
        &page_table::simple2,
        &page_table::free,
        &page_table::error,
        &page_table::complicate,
        &page_table::x86_permission,
        &page_table::x86_permission_advanced,
        // Mmap.
        &mm_struct::do_mmap,
        &mm_struct::access_ok_normal,
        &mm_struct::access_ok_invalid,
        &mm_struct::bad_addr_0,
        &mm_struct::get_user_page,
        // Loader.
        &userprog::arg_parse,
        &userprog::loader_bss_sanity,
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
        &userprog::mm_mmap_error_bad_addr,
        &userprog::mm_mmap_error_bad_fd,
        &userprog::mm_mmap_error_protection,
        &userprog::mm_mmap_error_protection_exec,
        &userprog::mm_munmap,
        &userprog::mm_munmap2,
        &userprog::mm_munmap_error_bad_addr,
        &userprog::mm_munmap_error_double_free,
        &userprog::mm_munmap_error_unaligned,
        &userprog::mm_exit_cleanup_stress,
        &userprog::bad_addr_1,
        &userprog::bad_code_write,
    ]);
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {}
