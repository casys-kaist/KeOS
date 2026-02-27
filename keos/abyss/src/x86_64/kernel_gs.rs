//! Kernel GS (General Segment)
//!
//! Used for internal data structure per CPU regardless of the running thread.

use super::{intrinsics::cpuid, msr::Msr, segmentation::TSS, tss::TaskStateSegment};
use crate::MAX_CPU;

#[repr(C, align(1024))]
#[derive(Clone, Debug)]
pub struct KernelGS {
    pub tss_addr: *const TaskStateSegment,
    __user_rbp_backup: usize,
    __r14_backup: usize,
    __r15_backup: usize,
    pub interrupt_frame: *const crate::interrupt::Registers,
}

const KERNEL_GS_INIT: KernelGS = KernelGS {
    tss_addr: core::ptr::null(),
    __user_rbp_backup: 0,
    __r14_backup: 0,
    __r15_backup: 0,
    interrupt_frame: core::ptr::null(),
};

pub unsafe fn current() -> &'static mut KernelGS {
    let ptr: *mut KernelGS;
    unsafe {
        ptr = Msr::<0xC000_0101>::read() as usize as *mut _;
        &mut *ptr as &mut KernelGS
    }
}

#[unsafe(no_mangle)]
pub static mut KERNEL_GS_BASES: [KernelGS; MAX_CPU] = [KERNEL_GS_INIT; MAX_CPU];

impl KernelGS {
    pub fn new() -> Self {
        let mut init = KERNEL_GS_INIT.clone();
        init.tss_addr = unsafe { &TSS[cpuid()].0 as *const TaskStateSegment };

        init
    }

    pub unsafe fn apply(self) {
        unsafe {
            KERNEL_GS_BASES[cpuid()] = self;
            Msr::<0xC000_0101>::write(&mut KERNEL_GS_BASES[cpuid()] as *mut KernelGS as u64);
        }
    }
}

impl Default for KernelGS {
    fn default() -> Self {
        Self::new()
    }
}
