//! The abyss of kernel that operates hardwares.
//!
//! This crate contains collections of hardware communications.
//!
//! You might see the codes of this code for your personal study for how x86_64
//! architecture cooperates with operating systems,
//! HOWEVER, most of codes in this crate exceeds the scope of the CS330
//! curriculum, which is why this crate is named "abyss".
//!
//! In other words, you do **NOT** have to see codes in this crate. Neither in
//! exams or projects, this crate is definitely **NOT** a coverage. Nonetheless,
//! if you want to go deeper into the low-level of the operating system, you may
//! see the internal of this crate with the [`OSDev Wiki`].
//!
//! **IN PARTICULAR, YOU ARE *NOT* SUPPOSED TO DIRECTLY USE THE MODULES OF
//! THIS CRATE TO IMPLEMENT THE KEOS PROJECT.**
//! We are not responsible for any problems occured by (mis)using codes of this
//! crate directly.
//!
//! Instead, you are supposed to see [`keos`] crate to see which modules (or
//! functions) are available for implementing KeOS Project.
//!
//! [`keos`]: ../keos/index.html
//! [`OSDev wiki`]: <https://wiki.osdev.org/Getting_Started>
#![no_std]
#![allow(internal_features, static_mut_refs, clippy::missing_safety_doc)]
#![feature(
    alloc_layout_extra,
    abi_x86_interrupt,
    core_intrinsics,
    lang_items,
    negative_impls,
    link_llvm_intrinsics
)]

use core::sync::atomic::AtomicBool;

extern crate alloc;

#[macro_use]
pub mod kprint;
pub mod addressing;
pub mod boot;
#[macro_use]
pub mod dev;
pub mod interrupt;
pub mod spinlock;
pub mod syscall;
pub mod unwind;
pub mod x86_64;

/// Maximum number of CPU the kernel can support.
pub const MAX_CPU: usize = 4;

#[doc(hidden)]
pub static QUITE: AtomicBool = AtomicBool::new(false);
