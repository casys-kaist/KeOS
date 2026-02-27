// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;
extern crate keos;
extern crate keos_project4;
#[macro_use]
extern crate grading;

mod cpuid;
mod hypercall;
mod msr;
mod pio;

use keos::SystemConfigurationBuilder;
use keos_project4::round_robin::RoundRobin;
use kev::Thread;

#[unsafe(no_mangle)]
pub unsafe fn main(config_builder: SystemConfigurationBuilder) {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
    config_builder.set_scheduler(RoundRobin::new());

    keos::TestDriver::<Thread>::start([
        &hypercall::hypercall_exit,
        &hypercall::hypercall_exit,
        &hypercall::hypercall_print,
        &pio::pio_print,
        &pio::pio_dx_port,
        &pio::pio_imm8_port,
        &pio::pio_mem,
        &cpuid::cpuid_leaf_0,
        &cpuid::cpuid_leaf_1,
        &msr::msr,
    ]);
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {
    unsafe { kev::start_vmx_on_cpu().expect("Failed to initialize VMX.") }
}

fn run_vm<const EXPECTED: i32>(code: &'static [u8]) {
    use kev::vm::VmBuilder;
    use kev_project1::no_ept_vm::NoEptVmState;

    let vm = VmBuilder::new(NoEptVmState::new(code), 1)
        .expect("Failed to create vmbuilder.")
        .finalize()
        .expect("Failed to create vm.");
    vm.start_bsp().expect("Failed to start bsp.");
    assert_eq!(vm.join(), EXPECTED);
}
