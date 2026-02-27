// This is test & bootstrap implementation.
// This file will be overwritten when grading.
#![no_std]
#![no_main]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;
extern crate keos;
extern crate keos_project2;
extern crate keos_project4;
#[macro_use]
extern crate grading;

mod page_table;
mod round_robin;

use keos::SystemConfigurationBuilder;
use keos_project4::round_robin::RoundRobin;
use kev::Thread;

#[unsafe(no_mangle)]
pub unsafe fn main(config_builder: SystemConfigurationBuilder) {
    config_builder.set_scheduler(RoundRobin::new());
    keos::TestDriver::<Thread>::start([
        // Page table.
        &page_table::simple,
        &page_table::simple2,
        &page_table::free,
        &page_table::error,
        &page_table::complicate,
        // Round robin Scheduler.
        &round_robin::functionality,
        &round_robin::balance,
        &round_robin::balance2,
        &round_robin::affinity,
    ]);
}

#[unsafe(no_mangle)]
pub unsafe fn ap_main() {}
