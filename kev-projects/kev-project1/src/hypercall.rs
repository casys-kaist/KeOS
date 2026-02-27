//! Hypercalls for project 2.
//!
//! Hypercall is a software trap from the guest operating system to hypervisor,
//! similar to the syscall from the application to kernel. You can simply think
//! hypercall as a "syscall" of the hypervisor and guest OS.
//!
//! In x86_64, guest OS can requests hypercall through the special instruction
//! "vmcall". When guest OS executes "vmcall" instruction, it first vmexits to
//! the hypervisor. After that, hypercall reads the registers and resolve the
//! requested hypercall according to the its own abi for hypercall.
//! And the hypervisor serves the requests and pass the control back to the
//! guest OS through the "vmresume" instruction.
//!
//! Both the project now and the project afterwards, you will use the following
//! abis for hypercall. %rax holds the hypercall number.
//! %rdi, %rsi, %rdx, %r10, %r9, %r8 are the first and second arguments, and so
//! on.
//!
//! ## Hypercall interface
//! The core interface of hypercall is [`HypercallAbi`] and [`Hypercall`]
//! traits. When the vcpu executes the vmcall, it traps into the vmexit handler
//! of the host operating system. Then the vmexit control infrastructure of the
//! kev forwards the given request to the [`Controller`] for the hypercall. When
//! the [`Controller`] found that the given request is a hypercall, it probes
//! the CPU state and resolve the information of the request through the
//! [`Hypercall::resolve`]. After that the [`Controller`] passes the decoded
//! hypercall request to the [`HypercallAbi::handle`]. The
//! [`HypercallAbi::handle`] then finally handles the given requests.
//!
//! ## Tasks
//! For this part, you are required to implement two hypercalls: the first halts
//! the current vCPU, while the second prints a string to the console.
//! The detailed Application Binary Interface (ABI) for each hypercall can be
//! founded in the [`Hypercall`] code section. When you write to the console,
//! you **MUST** proxy the console output through the [`PrinterProxy`].
//! Otherwise, grading script may be failed.
//!
//! [`HypercallAbi`]: crate::vmexit::hypercall::HypercallAbi
//! [`HypercallAbi::handle`]: crate::vmexit::hypercall::HypercallAbi::handle
//! [`Hypercall`]: crate::vmexit::hypercall::Hypercall
//! [`Hypercall::resolve`]: crate::vmexit::hypercall::Hypercall::resolve
//! [`Controller`]: crate::vmexit::hypercall::Controller
//! [`VmexitController`]: kev::vmexits::VmexitController
//! [`ExitReason`]: kev::vmcs::ExitReason
//! [`VmOps`]: kev::vm::VmOps
use crate::vmexit::hypercall;
use kev::{
    Probe, VmError,
    vcpu::{GenericVCpuState, VmexitResult},
    vm::Gva,
};

/// Hypercall context.
pub struct HypercallCtx;
impl hypercall::HypercallAbi for HypercallCtx {
    type Call = Hypercall;

    fn handle<P: Probe>(
        &mut self,
        hc: Self::Call,
        p: &mut P,
        GenericVCpuState { vmcs, vm, .. }: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        // Hint:
        //   - You can delegate exit request to the kev by returning
        //     `VmexitResult::Exited(0)`. You MUST not exit thread with
        //     [`keos::thread::with_current`] (Possibly leads deadlock.)
        //   - You can request vm to be exited by using trait [`kev::vm::VmOp`].
        //   - You can get &str through `core::str::from_utf8` and
        //     `core::slice::from_raw_parts`.
        //   - You MUST use write!(&mut crate::PrinterProxy, "{}", b) when writing to
        //     buffer.
        todo!()
    }
}

/// Supported hypercalls.
#[derive(Debug)]
pub enum Hypercall {
    /// Halt the vcpu with exitcode `code`.
    ///
    /// rax = 0.
    HaltCpu {
        /// Exit code. Provides on rdi.
        code: usize,
    },
    /// Print the message to the console.
    ///
    /// rax = 1.
    Print {
        /// Buffer to print. Provides on rdi.
        buf: Gva,
        /// Size of buffer to print. Provides on rsi.
        size: usize,
    },
}

impl hypercall::Hypercall for Hypercall {
    fn resolve(GenericVCpuState { gprs, .. }: &mut GenericVCpuState) -> Option<Self> {
        todo!()
    }
}
