use kev::{
    Probe, VmError,
    vcpu::{GenericVCpuState, msr},
    vm::Gpa,
};
use kev_project1::vmexit::msr::Msr;

#[derive(Default)]
pub struct KvmSystemTimeNew;

impl Msr for KvmSystemTimeNew {
    fn rdmsr(
        &self,
        _index: u32,
        _p: &dyn Probe,
        _generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<u64, VmError> {
        unreachable!()
    }

    fn wrmsr(
        &mut self,
        _index: u32,
        value: u64,
        p: &dyn Probe,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<(), VmError> {
        unsafe {
            msr::Msr::<0x4b564d01>::write(
                p.gpa2hpa(&generic_vcpu_state.vmcs, Gpa::new(value as usize).unwrap())
                    .unwrap()
                    .into_usize() as u64,
            );
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct KvmSystemTime;

impl Msr for KvmSystemTime {
    fn rdmsr(
        &self,
        _index: u32,
        _p: &dyn Probe,
        _generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<u64, VmError> {
        unreachable!()
    }

    fn wrmsr(
        &mut self,
        _index: u32,
        value: u64,
        p: &dyn Probe,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<(), VmError> {
        unsafe {
            msr::Msr::<0x12>::write(
                p.gpa2hpa(&generic_vcpu_state.vmcs, Gpa::new(value as usize).unwrap())
                    .unwrap()
                    .into_usize() as u64,
            );
        }
        Ok(())
    }
}
