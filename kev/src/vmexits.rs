//! Interface to play with vmexits.
use crate::{
    VmError,
    probe::Probe,
    vcpu::{GenericVCpuState, VmexitResult},
    vmcs::ExitReason,
};

/// Controller that defines action on vmexit.
pub trait VmexitController {
    /// Handle the vmexit on this controller.
    ///
    /// Returns [`VmError::HandleVmexitFailed`] when failed to handle vmexit on
    /// this controller.
    fn handle<P: Probe>(
        &mut self,
        reason: ExitReason,
        p: &mut P,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError>;
}

impl VmexitController for () {
    fn handle<P: Probe>(
        &mut self,
        _reason: ExitReason,
        _p: &mut P,
        _generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        Err(VmError::HandleVmexitFailed(_reason))
    }
}

impl<A: VmexitController, B: VmexitController> VmexitController for (A, B) {
    fn handle<P: Probe>(
        &mut self,
        reason: ExitReason,
        p: &mut P,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        let (a, b) = self;
        match a.handle(reason, p, generic_vcpu_state) {
            Err(VmError::HandleVmexitFailed(reason)) => b.handle(reason, p, generic_vcpu_state),
            r => r,
        }
    }
}
