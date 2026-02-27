// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;
extern crate keos;
extern crate keos_project4;
extern crate grading;

mod ept;
mod gkeos;
mod mmio;

use keos::SystemConfigurationBuilder;
use keos_project4::round_robin::RoundRobin;
use kev::Thread;

#[unsafe(no_mangle)]
pub unsafe fn main(config_builder: SystemConfigurationBuilder) {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
    config_builder.set_scheduler(RoundRobin::new());

    if let Ok(fs) = simple_fs::FileSystem::load(1) {
        keos::info!("Filesystem: use `SimpleFS`.");
        keos::fs::FileSystem::register(fs)
    }
    keos::TestDriver::<Thread>::start([
        &ept::simple,
        &ept::complicate,
        &ept::check_huge_translation,
        &mmio::mmio_print,
        &gkeos::run_keos,
    ]);
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
}
