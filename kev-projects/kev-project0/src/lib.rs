//! Project0: KeOS.
//!
//! In this project, you will implement the selective component of `KeOS` (KAIST educational operating system),
//! which is nessary to run the KeV hypervisor.
//!
//! This project is divided into two sections: [`Round-robin Scheduling`] and [`Page Table`].
//! If you already completed the whole keos projects, you can skip this project and use that implementations.
//!
//! ## Getting started
//!
//! ```/bin/bash
//! $ cargo run --target ../.cargo/x86_64-unknown-keos.json
//! ```
//!
//! ## Outline
//! - [`Round-robin Scheduling`]
//! - [`Page Table`]
//!
//! [`Round-robin Scheduling`]: <https://casys-kaist.github.io/KeOS/keos_project4/round_robin/index.html>
//! [`Page Table`]: <https://casys-kaist.github.io/KeOS/keos_project2/page_table/index.html>

#![no_std]

extern crate alloc;
#[allow(unused_imports)]
#[macro_use]
extern crate keos;