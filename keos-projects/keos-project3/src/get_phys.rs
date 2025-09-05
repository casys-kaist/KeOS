//! # A utility system call for grading.

#![doc(hidden)]

use keos::{
    KernelError,
    addressing::{PAGE_MASK, Va},
};
use keos_project1::{file_struct::FileStruct, syscall::SyscallAbi};
use keos_project2::mm_struct::MmStruct;

use crate::lazy_pager::LazyPager;

#[doc(hidden)]
pub fn get_phys(
    mm: &MmStruct<LazyPager>,
    fs: &FileStruct,
    abi: &SyscallAbi,
) -> Result<usize, KernelError> {
    if abi.arg2 == 0x80041337 {
        Ok(fs as *const _ as usize)
    } else {
        let va = Va::new(abi.arg1)
            .ok_or(KernelError::InvalidArgument)?
            .page_down();
        let pte = mm
            .page_table
            .walk(va)
            .ok()
            .ok_or(KernelError::InvalidArgument)?;

        Ok(if abi.arg2 == 0 {
            pte.pa().ok_or(KernelError::InvalidArgument)?.into_usize() | abi.arg1 & PAGE_MASK
        } else {
            pte.flags().bits()
        })
    }
}
