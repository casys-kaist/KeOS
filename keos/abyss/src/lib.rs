//! The abyss of kernel that operates hardwares.
//!
//! This crate contains collections of hardware communications.
//!
//! You might see the codes of this code for your personal study for how x86_64
//! architecture cooperates with operating systems,
//! HOWEVER, most of codes in this crate exceeds the scope of the CS330
//! curriculum, which is why this crate is named "abyss".
//!
//! In other words, you are **not** required to understand every line of code in
//! this crate. Most of its implementation details are not directly covered in
//! exams or projects. However, some parts that you will explicitly use during
//! the implementation (e.g., x86 register related contents)
//! **may be** included in exams.
//!
//! If you want to go deeper into the low-level of the operating system, you may
//! explore the internals of this crate along with the [`OSDev Wiki`].
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

#[doc(hidden)]
#[macro_use]
pub mod kprint;
#[doc(hidden)]
pub mod addressing;
#[doc(hidden)]
pub mod boot;
#[doc(hidden)]
#[macro_use]
pub mod dev;
#[doc(hidden)]
pub mod interrupt;
#[doc(hidden)]
pub mod spinlock;
#[doc(hidden)]
pub mod syscall;
#[doc(hidden)]
pub mod unwind;
#[doc(hidden)]
pub mod x86_64;

#[cfg(doc)]
pub use addressing::{Pa, Va};
#[cfg(doc)]
pub use interrupt::GeneralPurposeRegisters;
#[cfg(doc)]
pub use interrupt::Registers;
#[cfg(doc)]
pub use spinlock::SpinLock;
#[cfg(doc)]
pub use x86_64::interrupt::PFErrorCode;

/// Maximum number of CPU the kernel can support.
pub const MAX_CPU: usize = 4;

#[doc(hidden)]
pub static QUITE: AtomicBool = AtomicBool::new(false);
