//! Project 3: Hardware Virtualization.
//!
//! In this project, you will implement the hardware-based virtualization and
//! run guest KeOS (gKeOS) on KeV hypervisor.
//! The term "hardware-based virtualization" refers to the process of
//! virtualizing platform hardware to run various operating systems on single
//! hardware with hardware supports. Through this, users can run multiple
//! operating systems on a single hardware platform as if they were running on
//! multiple physical machines with minimal performance degradation.
//!
//! ## Background
//! ### Hardware-based virtualization
//! Hardware-based virtualization refers to the method of providing a virtual
//! computing environment through the use of hardware, typically required for
//! the operations of the virtual machines. This is accomplished using
//! technologies such as Intel-VT or AMD-V, which provide the ability to run
//! codes on another privilege level, enabling full virtualization without the
//! need for OS modification such as binary translation or para-virtualization,
//! with little or minimum performance degradations.
//!
//! Although hardware-based virtualization supports many hardware features,
//! trap-and-emulate is still required as it is difficult to virtualize all
//! instructions through the hardware. Therefore, the hardware assists in
//! virtualization by focusing on frequently occurring or performance-critical
//! instructions. One such example is page table modifications. In the past,
//! shadow page table was used to emulate every memory access permission for
//! each page table entry, but with the hardware-based virtualization, extended
//! page table is used to reduce MMU overhead by allowing the hardware to
//! perform these permission checks on behalf of the software.
//!
//! ## Getting started
//! When you run the following command lines in the project3 directory, keos
//! will panic with "not yet implemented" message. 
//! 
//! ```/bin/bash
//! $ cargo run --target ../.cargo/x86_64-unknown-keos.json
//! ```
//! 
//! ## Outline
//! - [`Extended Page Table`]
//! - [`Memory-mapped I/O`]
//! - [`Lazy paging`]
//!
//! [`Extended Page Table`]: ept
//! [`Memory-mapped I/O`]: mmio
//! [`Lazy paging`]: keos_vm/pager

#![no_std]
#![feature(new_zeroed_alloc)]

extern crate alloc;
#[allow(unused_imports)]
#[macro_use]
extern crate keos;

pub mod ept;
pub mod keos_vm;
pub mod mmio;
pub mod simple_ept_vm;

pub mod vmexit {
    #[path = "mmio.rs"]
    pub mod mmio;
}
