// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate keos;
extern crate keos_project2;
extern crate keos_project4;
#[macro_use]
extern crate grading;

mod round_robin;
mod simple_virtio;
mod virtio;

use keos::SystemConfigurationBuilder;
use keos_project4::round_robin::RoundRobin;

/// A Thread for gKeOS.
#[derive(Default)]
pub struct Thread {}

impl keos::task::Task for Thread {
    fn syscall(&mut self, _registers: &mut keos::syscall::Registers) {
        unreachable!()
    }
}


#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe fn main(config_builder: SystemConfigurationBuilder) {
    config_builder.set_scheduler(RoundRobin::new());
    keos::TestDriver::<Thread>::start([
        &virtio::check_blockio,
        &virtio::check_blockio_batching,
        &round_robin::functionality,
        &round_robin::balance,
        &round_robin::balance2,
        &round_robin::affinity,
    ]);
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {}
