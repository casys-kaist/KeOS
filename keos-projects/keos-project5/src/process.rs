//! The process model for project5.
//!
//! This file defines the process model of the project5.

use keos_project1::file_struct::FileStruct;
use keos_project2::mm_struct::MmStruct;
use keos_project3::lazy_pager::LazyPager;

/// A thread state of project 5, which contains file and memory state.
#[repr(transparent)]
#[derive(Default)]
pub struct Thread(pub keos_project4::Thread);

impl core::ops::Deref for Thread {
    type Target = keos_project4::Thread;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for Thread {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Thread {
    pub fn from_mm_struct(mm_struct: MmStruct<LazyPager>, tid: u64) -> Self {
        Self(keos_project4::Thread::from_mm_struct(mm_struct, tid))
    }

    pub fn from_fs_mm_struct(
        file_struct: FileStruct,
        mm_struct: MmStruct<LazyPager>,
        tid: u64,
    ) -> Self {
        Self(keos_project4::Thread::from_file_mm_struct(
            file_struct,
            mm_struct,
            tid,
        ))
    }
}
