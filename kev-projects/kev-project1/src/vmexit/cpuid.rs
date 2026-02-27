//! Cpuid vmexit controller.
//!
//! The cpuid instruction is a special instruction in x86 processors, used to
//! obtain information about the processor and its features. When executed, the
//! instruction returns data in the processor's registers, including the vendor,
//! model number, and feature flags.
//!
//! The cpuid instruction takes one input argument (leaf) in the EAX register,
//! which determines the type of information to retrieve. Different input values
//! provide different information. For example, setting EAX to 0 returns a
//! 12-character ASCII string representing the CPU manufacturer ID in EBX, EDX,
//! and ECX registers, with "GenuineIntel" being the string for Intel
//! processors.
//!
//! A guest operating system cannot execute the Cpuid instruction directly,
//! instead causing a VM exit to the host, which must handle the instruction.
//! This part aims to emulate the cpuid instruction in the guest by executing
//! the instruction on the host side. However, not all the result could be
//! forwarded. For example, when the host executes the Cpuid instruction with
//! EAX = 1, the result contains the executing core's CPU ID, not the VCPU ID.
//! In this case, the result needs to modified to contain the VCPU ID.
//!
//! ## Tasks
//! Implement cpuid controller's handle method to emulate cpuid instruction.
//! If the input to the instruction is EAX = 1, you must carefully handle the
//! cpuid. Because it holds the cpu id of the current logical processor not
//! virtual cpu id. It may be helpful to understand [how to obtain the CPU ID of
//! the executing core.](/src/abyss/x86_64/intrinsics.rs.html) In addition, you
//! **MUST** forward the vCPU instruction pointer (rip) to prevent it from
//! executing the same instructions indefinitely.
use kev::{
    Probe, VmError,
    vcpu::{GenericVCpuState, VmexitResult},
    vmcs::{BasicExitReason, ExitReason},
};

/// Cpuid vmexit controller.
pub struct Controller {}

impl Controller {
    /// Create a new cpuid controller.
    pub fn new() -> Self {
        Self {}
    }
}

impl kev::vmexits::VmexitController for Controller {
    fn handle<P: Probe>(
        &mut self,
        reason: ExitReason,
        _p: &mut P,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        match reason.get_basic_reason() {
            BasicExitReason::Cpuid => {
                // HINT:
                //    - Use `core::arch::x86_64::__cpuid` to execute `cpuid`.
                //    - You should advance rip when an instruction is emulated.
                //    - You must carefully handle the cpuid leaf 1. Because it holds the cpu id,
                //      you must change the value to the virtual cpu id.
                todo!()
            }
            _ => Err(kev::VmError::HandleVmexitFailed(reason)),
        }
    }
}
