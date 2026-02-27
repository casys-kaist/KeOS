// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;
extern crate grading;
extern crate keos;
extern crate keos_project4;

use keos::SystemConfigurationBuilder;
use keos_project4::round_robin::RoundRobin;
use kev::{Thread, vm::VmBuilder};
use kev_project3::vm::VmState;

#[unsafe(no_mangle)]
pub unsafe fn main(config_builder: SystemConfigurationBuilder) {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
    config_builder.set_scheduler(RoundRobin::new());

    if let Ok(fs) = simple_fs::FileSystem::load(1) {
        keos::info!("Filesystem: use `SimpleFS`.");
        keos::fs::FileSystem::register(fs)
    }
    keos::TestDriver::<Thread>::start([
        &run_keos,
    ]);
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
}

pub fn run_keos() {
    // VM with 256 MiB memory.
    let vm = VmBuilder::new(
        VmState::new(256 * 1024).expect("Failed to crate vmstate"),
        4,
    )
    .expect("Failed to create vmbuilder.")
    .finalize()
    .expect("Failed to create vm.");
    vm.start_bsp().expect("Failed to start bsp.");
    vm.join();
}
