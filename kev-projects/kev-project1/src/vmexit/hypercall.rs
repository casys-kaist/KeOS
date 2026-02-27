//! Hypercall vmexit controller.
use alloc::boxed::Box;
use kev::{
    Probe, VmError,
    vcpu::{GenericVCpuState, VmexitResult},
    vmcs::{BasicExitReason, ExitReason},
};

/// Hypercall vmexit controller.
pub struct Controller<H: HypercallAbi> {
    inner: H,
}

impl<H: HypercallAbi> Controller<H> {
    /// Create a new hypercall controller.
    pub fn new(inner: H) -> Self {
        Self { inner }
    }
}

impl<H: HypercallAbi> kev::vmexits::VmexitController for Controller<H> {
    fn handle<P: Probe>(
        &mut self,
        reason: ExitReason,
        p: &mut P,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        match reason.get_basic_reason() {
            BasicExitReason::Vmcall => {
                let hc = H::Call::resolve(generic_vcpu_state)
                    .ok_or(VmError::ControllerError(Box::new("Unknown hypercall")))?;
                self.inner
                    .handle(hc, p, generic_vcpu_state)
                    .and_then(|r| generic_vcpu_state.vmcs.forward_rip().map(|_| r))
            }
            _ => Err(kev::VmError::HandleVmexitFailed(reason)),
        }
    }
}

/// Trait that represent the hypercall abi.
pub trait HypercallAbi
where
    Self: Sync + Send + 'static,
{
    /// Hypercalls that this controller can handle.
    type Call: Hypercall;

    /// Handle the hypercall `hc`.
    fn handle<P: Probe>(
        &mut self,
        hc: Self::Call,
        p: &mut P,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, kev::VmError>;
}

/// Trait that represent the enumeration of supported hypercall.
pub trait Hypercall {
    /// Resolve the requested hypercall.
    fn resolve(generic_vcpu_state: &mut GenericVCpuState) -> Option<Self>
    where
        Self: Sized;
}
