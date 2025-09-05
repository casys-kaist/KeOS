// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc)]
#![feature(slice_as_array)]

extern crate alloc;
extern crate grading;
extern crate keos;
extern crate keos_project1;
extern crate keos_project2;
extern crate keos_project3;
extern crate keos_project4;
extern crate keos_project5;

pub mod ffs;
pub mod ffs_no_journal;
pub mod journal;
pub mod page_cache;
pub mod syscall_part_2;
pub mod userprog;

use keos::{SystemConfigurationBuilder, fs::Disk};
use keos_project4::round_robin::RoundRobin;
use keos_project5::{Thread, page_cache::PageCache};

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe fn main(config_builder: SystemConfigurationBuilder) {
    config_builder.set_scheduler(RoundRobin::new());
    if let Ok(fs) = keos_project5::ffs::FastFileSystem::from_disk(Disk::new(2), true, false) {
        keos::info!("Filesystem: use `FastFileSystem` with `PageCache`.");
        keos::fs::FileSystem::register(PageCache::new(fs))
    } else {
        panic!("FFS is not available");
    }
    keos::TestDriver::<Thread>::start([
        /* Page Cache Tests */
        &page_cache::simplefs,
        &page_cache::readahead,
        /* FFS Functionality Tests */
        &ffs::root,
        &ffs::root_open_self,
        &ffs::root_open_absent,
        &ffs::add_file,
        &ffs::ib,
        &ffs::dib,
        &ffs::add_directory,
        &ffs::file_in_dir,
        &ffs::remove_file,
        &ffs::read_dir,
        &ffs::remove_dir,
        &ffs::remove_root,
        &ffs::simple_elf,
        /* Page Cache + FFS Tests */
        &page_cache::fastfilesystem,
        &page_cache::readahead_ffs,
        &page_cache::writeback,
        /* FS1 Directory primitive syscall tests */
        &syscall_part_2::open_dir,
        &syscall_part_2::dir_rw,
        &syscall_part_2::dir_seek,
        /* Directory system call tests (basic) */
        &syscall_part_2::create,
        &syscall_part_2::unlink,
        &syscall_part_2::chdir,
        /* FFS Journaling Tests */
        &journal::recovery,
        /* FFS Functionality with Journaling Tests */
        &ffs_no_journal::root,
        &ffs_no_journal::root_open_self,
        &ffs_no_journal::root_open_absent,
        &ffs_no_journal::add_file,
        &ffs_no_journal::ib,
        &ffs_no_journal::dib,
        &ffs_no_journal::add_directory,
        &ffs_no_journal::file_in_dir,
        &ffs_no_journal::remove_file,
        &ffs_no_journal::read_dir,
        &ffs_no_journal::remove_dir,
        &ffs_no_journal::remove_root,
        &ffs_no_journal::simple_elf,
        /* User Program */
        &userprog::sha256sum,
        &userprog::ls,
        &userprog::tar,
        &userprog::tar_gen,
    ]);
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {}
