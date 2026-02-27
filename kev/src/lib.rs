//! Welcome to the KeV project.
//!
//! Virtualization is an increasingly ubiquitous feature of modern computer
//! systems, and a rapidly evolving part of the system stack. Hardware vendors
//! are adding new features to support more efficient virtualization, OS designs
//! are adapting to perform better in VMs, and VMs are an essential component in
//! cloud computing. Thus, understanding how VMs work is essential to a complete
//! education in computer systems.
//!
//! In this project, you will skim through the basic components that runs on
//! real virtual machine monitor like KVM. From what you learn, you will build
//! your own type 2 hypervisor and finally extend the hypervisor
//! as an open-ended course project.
//!
//! In KeV project, we will not bother you from the time-consuming edge case
//! handling and the hidden test cases. The score that you see when run the
//! grading scripts is your final score. We want to keep this project as easy as
//! possible. If you have suggestions on how we can reduce the unnecessary
//! overhead of assignments, cutting them down to the important underlying
//! issues, please let us know.
//!
//! ## Projects
//! The KeV project consists of 5 projects.
//!
//! 1. [KeOS]
//! 2. [VMCS and VMexits]
//! 3. [Hardware virtualization]
//! 4. [Interrupt and I/O virtualization]
//! 5. [Final project]
//!
//! ### Rust
//! We pick the Rust as a language for project. This is because we believe that
//! after overcome the barriers to learn, memory safety and ownership rule of
//! Rust could significantly reduce the debugging time while implement an
//! operating system.
//!
//! ## Getting Started
//! You can bootstrap your KeV project with following command lines:
//! ```bash
//! $ mkdir keos
//! $ cd keos
//! $ curl https://raw.githubusercontent.com/casys-kaist/KeOS/refs/heads/main/scripts/install-kev.sh | sh
//! ```
//!
//! **PLEASE DO NOT MAKE ANY PUBLIC FORK OF THIS PROJECT.**
//! This is strongly denied from the license of the KeV Project. You **MUST**
//! not redistribute the your works based on the given template.
//!
//! ### Enable nested virtualization
//!
//! See the following docs: <https://docs.fedoraproject.org/en-US/quick-docs/using-nested-virtualization-in-kvm/>
//!
//!
//! ### Additional Resources
//!
//! Refer to the main [KeOS documentation] for more details on general workflow
//! of KeV projects, including grading, implementation tips and debugging.
//!
//! [KeOS documentation]: ../keos
//! [KeOS]: ../kev-projects/kev-project0
//! [VMCS and VMexits]: ../kev-projects/kev-project1
//! [Hardware virtualization]: ../kev-projects/kev-project2
//! [Interrupt and I/O virtualization]: ../kev-projects/kev-project3
//! [Final project]: ../kev-projects/kev-project4

#![no_std]
#![feature(get_mut_unchecked)]
#![deny(missing_docs)]

extern crate alloc;
#[macro_use]
extern crate keos;

mod probe;
pub mod vcpu;
pub mod vm;
pub mod vm_control;
#[allow(dead_code)]
pub mod vmcs;
pub mod vmexits;

use abyss::x86_64::{Cr0, Cr4, msr::Msr};
use alloc::boxed::Box;
use keos::{interrupt::register, intrinsics::cpuid, syscall::Registers, task::Task};
pub use probe::Probe;
use vm_control::*;
use vmcs::{ExitReason, Vmcs};

#[doc(hidden)]
pub trait Bits {
    fn bit_test(self, index: usize) -> bool;
}

impl Bits for u32 {
    fn bit_test(self, index: usize) -> bool {
        (self >> index) & 1 != 0
    }
}

impl Bits for u64 {
    fn bit_test(self, index: usize) -> bool {
        (self >> index) & 1 != 0
    }
}

/// Possible errorkind for Vmx.
#[derive(Debug)]
pub enum VmxError {
    /// Virtual-machine eXtension is not supported.
    VmxNotSupported,
    /// Ept is not supported.
    EptNotSupported,
    /// Current Cr0 value is invalid.
    InvalidCr0,
    /// Current Cr4 value is invalid.
    InvalidCr4,
    /// Vmx is disabled in bios.
    InvalidBiosConfig,
    /// Vmcs operation has an error.
    VmxOperationError(vmcs::InstructionError),
}

/// Possible errorkind for Vm.
#[derive(Debug)]
pub enum VmError {
    /// Vm operation has error.
    VmxOperationError(vmcs::InstructionError),
    /// Failed to handle vmexit.
    HandleVmexitFailed(ExitReason),
    /// Controller-private error.
    ControllerError(Box<dyn core::fmt::Debug + Send + Sync>),
    /// Failed to decode instruction.
    FailedToDecodeInstruction,
    /// Vcpu related error.
    VCpuError(Box<dyn core::fmt::Debug + Send + Sync>),
}

/// Enable the VM-eXtension on this cpu.
///
/// # Safety
/// Must be called once on each cpu.
pub unsafe fn start_vmx_on_cpu() -> Result<(), VmxError> {
    unsafe {
        (Cr4::current() | Cr4::VMXE).apply();
        // Load vmx realated msrs.
        let (vmx_cr0_fixed_0, vmx_cr0_fixed_1, vmx_cr4_fixed_0, vmx_cr4_fixed_1) = (
            Cr0::from_bits_truncate(Msr::<IA32_VMX_CR0_FIXED0>::read()),
            Cr0::from_bits_truncate(Msr::<IA32_VMX_CR0_FIXED1>::read()),
            Cr4::from_bits_truncate(Msr::<IA32_VMX_CR4_FIXED0>::read()),
            Cr4::from_bits_truncate(Msr::<IA32_VMX_CR4_FIXED1>::read()),
        );
        let (cr0, cr4) = (Cr0::current(), Cr4::current());
        // Intel® 64 and IA-32 Architectures Software Developer’s Manual.
        // 23.8 RESTRICTIONS ON VMX OPERATION
        if (vmx_cr0_fixed_1 | cr0 != vmx_cr0_fixed_1) || !cr0 & vmx_cr0_fixed_0 != Cr0::empty() {
            return Err(VmxError::InvalidCr0);
        }
        if (vmx_cr4_fixed_1 | cr4 != vmx_cr4_fixed_1) || !cr4 & vmx_cr4_fixed_0 != Cr4::empty() {
            return Err(VmxError::InvalidCr4);
        }

        // Intel® 64 and IA-32 Architectures Software Developer’s Manual.
        // 6.2.1 Detecting and Enabling SMX

        // Try to enable VMX outside SMX operation.
        let feature_control = Msr::<IA32_FEATURE_CONTROL>::read();
        if !feature_control.bit_test(2) {
            Msr::<IA32_FEATURE_CONTROL>::write(feature_control | (1 << 2));
            if !feature_control.bit_test(2) {
                return Err(VmxError::InvalidBiosConfig);
            }
        }

        // Try to lock.
        // Lock bit (0 = unlocked, 1 = locked). When set to '1' further writes to this
        // MSR are blocked
        let feature_control = Msr::<IA32_FEATURE_CONTROL>::read();
        if !feature_control.bit_test(0) {
            Msr::<IA32_FEATURE_CONTROL>::write(feature_control | (1 << 0));
        }

        // Intel® 64 and IA-32 Architectures Software Developer’s Manual.
        // 23.6 DISCOVERING SUPPORT FOR VMX
        if !core::arch::x86_64::__cpuid(1).ecx.bit_test(5) {
            return Err(VmxError::VmxNotSupported);
        } else if !Msr::<IA32_VMX_PROC_BASED_CTLS>::read().bit_test(63)
            || !Msr::<IA32_VMX_PROC_BASED_CTLS>::read().bit_test(33)
        {
            return Err(VmxError::EptNotSupported);
        }

        if cpuid() == 0 {
            register(100, |_| {});
        }

        core::mem::ManuallyDrop::new(Box::new(Vmcs::new()))
            .on()
            .map_err(VmxError::VmxOperationError)
    }
}

/// A Thread for KeV projects.
#[derive(Default)]
pub struct Thread {}

impl Task for Thread {
    fn syscall(&mut self, _registers: &mut Registers) {
        unreachable!()
    }
}
