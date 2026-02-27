// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate keos;

use keos::SystemConfigurationBuilder;

#[unsafe(no_mangle)]
pub unsafe fn main(_config_builder: SystemConfigurationBuilder) {
    println!("Hello guest os!");

    // Hypercall exit.
    unsafe {
        core::arch::asm!("xor rax, rax", "mov rdi, 0", "vmcall");
    }
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {}
