//! Project 1: VMCS and VMExits.
//!
//! In the first project, you will implement the basic building blocks of the
//! virtualization. Although the hardware-based virtualization gives a safe and
//! convinient way to support virtualize the machine, it is not a perfect tool.
//!
//! Some less-frequently used and safety sensitive operations are required to be
//! trap-and-emulated. Examples of such sensitive instructions include cpuid,
//! port-mapped I/O, and memory-mmaped I/O.
//!
//! Some operations have high cost to trap-and-emulate multiple steps, they are
//! optimization that combines multiple operations into a single operation
//! called hypercall.
//!
//! You will learn about multiple trap-and-emulate operations by implementing a
//! simple print function in three different ways: hypercall, port-mapped I/O
//! and memory-mapped I/O. In project2, you will play on first two, hypercall
//! and port-mapped I/O.
//!
//! Also, you will learn the concepts of instruction emulation.
//! Some instructions (e.g. cpuid, rdmsr, wrmsr) are sensitive to run directly
//! on guest cpu or required some modifications. The VMM requires to
//! trap-and-emulate those instructions.
//!
//! While hands-on the project, you will become familiar with kev's
//! infrastructure and the concepts of VMCS and VMExit.
//!
//! ## Background
//! ### Virtual machine
//! A virtual machine is an physical computer system that provides virtualized
//! hardware resources. The virtual resources, such as CPU, memory, storage, and
//! network interfaces, are presented to guest operating systems and
//! applications as if they were physically present. By running on top of a
//! virtual machine, normal programs and operating systems can be executed using
//! these virtual resources.
//!
//! Multiple virtual machines can be operated on a single physical machine,
//! and the distribution of physical resources to virtual machines is managed by
//! a software layer called a Virtual Machine Monitor (VMM), also known as a
//! Hypervisor. The VMM provides a layer of abstraction between the virtual
//! machines and the underlying physical hardware, enabling physical resources
//! to be shared among all VMs. Each VM environment is isolated from other VMs,
//! so they do not know about each other and are unaware that they are
//! controlled by the VMM.
//!
//! Modern CPU architectures, such as those from Intel and AMD, provide hardware
//! virtualization support that enables system engineers to design custom
//! virtual machine environments. By utilizing hardware-assisted virtualization,
//! guest virtual machines can securely and efficiently access physical
//! resources provided by the host machine. In the KeV projects, we will
//! utilizes the Intel's hardware virtualization, called Intel VM-eXtension
//! (Intel VMX).
//!
//! ### VM Exit
//! Hardware virtualization is achieved by dividing the execution context into
//! two modes: host (VMX-root mode in Intel VMX) and guest (VMX-non-root mode in
//! Intel VMX). The VM Exit and VM Entry events facilitate mode switching
//! between these two contexts. When a virtual machine operation is executed in
//! non-root mode, certain sensitive instructions or events may trigger a VM
//! exit, which transfers control from the guest os context to the host os
//! context.
//!
//! When a VM exit occurs, the VMM takes control and decides how to handle the
//! sensitive operation or event. The VMM may emulate the operation, modify the
//! operation to suit the virtualized environment, or defer the operation to the
//! underlying hardware. After the operation is handled, the VMM can choose to
//! resume execution in the non-root context.
//!
//! ### Virtual Machine Control data Structure (Intel VMX)
//! Virtual machine control data structures (VMCS) are structures used by x86
//! during virtual machine execution. VMCS stores detailed information on how a
//! virtual machine will operate and its internal states, including its CPU and
//! memory, IO configurations, interrupt handling, and other hardware settings.
//!
//! Virtual machine monitor (VMM) can control and read hardware features by
//! assigning difference VMCS to each virtual machine virtual processors.
//! For instance, a virtual machine with four virtual cores would use four VMCS
//! to control each core's functionality.
//!
//! To provide VMM with an abstract specification of the VMCS, which has
//! difference data structures across processor vendors and versions, hypervisor
//! access to VMCS is made through special instructions called `vmwrite` and
//! `vmread`. `vmwrite` writes to a specific field of the activated VMCS and
//! `vmread` reads a specific field of the activated VMCS. Therefore, x86
//! provides ways to access and distinguish which VMCS is currently being set by
//! the hypervisor. To this end, the VMCS states of Active, Current, and Clear
//! have been added. In this project, hardware and software details for VMCS
//! execution states and its associated instructions are hided from you.
//!
//! ## KeV
//! KeV supplies a layer to abstraction to access the hardware-specific details.
//! It provides abstractions of Virtual Machine ([`Vm`]) and virtual CPU
//! ([`VCpu`]).
//!
//! ### Vm
//! The [`Vm`] is an abstraction of a virtual machine. It contains multiple
//! virtual CPUs and its internal states. You can interact with the [`Vm`]
//! instance with the trait [`VmOps`], which defines the mutiple operations of a
//! Virtual Machine. See the [`VmOps`] for list of the operations.
//!
//! ### VCpu
//! The [`VCpu`] is an abstraction of a single virtual CPU. It holds the states
//! to run virtual CPU, including VMCS, general purpose registers, virtual CPU
//! ID, and VM Exit handlers. Each [`VCpu`] is an entity of scheudling. That is,
//! it is a basically a thread of a host operating system!
//!
//! The VCpu thread runs in a loop that launch the VCpu, and handle the VM Exit.
//! In the loop, the thread launches the guest operating system through
//! `vmlaunch` or `vmresume`. After that, guest os traps back to the vmm by VM
//! Exit, it resolves the reason of the VM Exit, handles the VM Exit, and
//! returns back to the guest operating system. You can found the details of the
//! loop on [`Vm::vcpu_thread_work`].
//!
//! ### Guest Memory Abstraction
//! With the introduction of the guest operating system, KeV brings two terms:
//! guest physical address ([`Gpa`]) and guest virtual addresss ([`Gva`]).
//!
//! In project 2, all you need to mind is that there is no guest physical
//! address and guest virtual address is same as a host virtual address because
//! we are not introduce the memory virtualization yet. We will revisit this
//! topics in project 3 with memory virtualiation.
//!
//! ### VmexitController
//! The [`VmexitController`] is the core interface to play with the VM Exits.
//! It is a trait define as follow:
//! ```rust
//! pub trait VmexitController {
//!    /// Handle the vmexit on this controller.
//!    ///
//!    /// Returns [`VmError::HandleVmexitFailed`] when failed to handle vmexit on this controller.
//!    fn handle<P: Probe>(
//!        &mut self,
//!        reason: ExitReason,
//!        p: &mut P,
//!        generic_vcpu_state: &mut GenericVCpuState,
//!    ) -> Result<VmexitResult, VmError>;
//! }
//! ```
//! The VmexitController trait requires four arguments as input parameters
//! * &mut self: Contains the self object of the given controller structure. You
//!   can holds controller-specific states in `self`.
//! * reason: Argument represents the [`ExitReason`] that holds the exit reasons
//!   for the current vm exit. For instance, if the current vcpu is a hypercall,
//!   the exit reasons would be [`BasicExitReason::Vmcall`].
//! * p: [`Probe`] object that holds an information of memory state of the guest
//!   operating system. You can use this object to translate address between
//!   guest and host operating system.
//! * generic_vcpu_state: Defines the states for the current vcpu. These states
//!   comprise the current Virtual Machine Control Structure (VMCS), general
//!   purpose registers (gen), and other relevant components which includes the
//!   [`VmOps`]. See the [`GenericVCpuState`].
//!
//!
//! #### ActiveVmcs
//! [`ActiveVmcs`] is an abstraction to communicate with the activated VMCS.
//! You can read or write to the activated VMCS with this struct.
//! This also defines multiple helper functions to modify the VMCS.
//!
//! For instance,
//! ```rust
//! /// Forward to the next instruction.
//! pub fn forward_rip(&self) -> Result<(), VmError> {
//!    self.write(
//!        Field::GuestRip,
//!        self.read(Field::GuestRip)? + self.read(Field::VmexitInstructionLength)?,
//!    )
//! }
//! ```
//! The VMCS in this code (&self) is capable of reading its internal fields to
//! read and write the RIP information necesasry for RIP forwarding. As like in
//! this example, you can read and write the VMCS structure for fetching
//! hardware informations of virtual machine in your KeV projects.
//!
//! ## Getting started
//! When you run following command lines in the project2 directory, keos will be
//! panic with "not yet implemented" message. 
//! 
//! ```/bin/bash
//! $ cargo run --target ../.cargo/x86_64-unknown-keos.json
//! ```
//! 
//! ## Outline
//! - [`Virtual Machine Control Structure (VMCS)`]
//! - [`Hypercall`]
//! - [`Port-mapped I/O`]
//! - [`Cpuid`]
//! - [`Model-specific Register`]
//!
//! [`Virtual Machine Control Structure (VMCS)`]: no_ept_vm
//! [`Hypercall`]: hypercall
//! [`Cpuid`]: vmexit/cpuid
//! [`Port-mapped I/O`]: vmexit/pio
//! [`Model-specific Register`]: vmexit/msr
//! [`ActiveVmcs`]: kev::vmcs::ActiveVmcs
//! [`Vm`]: kev::vm::Vm
//! [`VCpu`]: kev::vcpu::VCpu
//! [`VmOps`]: kev::vm::VmOps
//! [`Vm::vcpu_thread_work`]: kev::vm::Vm::vcpu_thread_work
//! [`Gpa`]: kev::vm::Gpa
//! [`Gva`]: kev::vm::Gva
//! [`VmexitController`]: kev::vmexits::VmexitController
//! [`ExitReason`]: kev::vmcs::ExitReason
//! [`BasicExitReason::Vmcall`]: kev::vmcs::BasicExitReason::Vmcall
//! [`GenericVCpuState`]: kev::vcpu::GenericVCpuState
//! [`Probe`]: kev::Probe
#![no_std]

extern crate alloc;
#[allow(unused_imports)]
#[macro_use]
extern crate keos;

pub mod no_ept_vm;

pub mod hypercall;
pub mod msr;
pub mod pio;
pub mod vmexit;
