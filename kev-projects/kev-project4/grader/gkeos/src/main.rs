// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]

extern crate alloc;
extern crate keos;

use keos::SystemConfigurationBuilder;

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe fn main(_config_builder: SystemConfigurationBuilder) {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {}
