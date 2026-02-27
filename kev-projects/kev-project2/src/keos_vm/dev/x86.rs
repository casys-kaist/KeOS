use kev::{
    Probe, VmError,
    vcpu::{GenericVCpuState, VmexitResult},
    vmcs::Field,
};
use kev_project1::vmexit::{
    msr::Msr,
    pio::{Direction, PioHandler},
};

#[derive(Default)]
pub struct EferMsr;

impl Msr for EferMsr {
    fn rdmsr(
        &self,
        _index: u32,
        _p: &dyn Probe,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<u64, VmError> {
        generic_vcpu_state.vmcs.read(Field::GuestIa32Efer)
    }

    fn wrmsr(
        &mut self,
        _index: u32,
        value: u64,
        _p: &dyn Probe,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<(), VmError> {
        generic_vcpu_state.vmcs.write(Field::GuestIa32Efer, value)
    }
}

// Address: 0xCF8.
// output: 0xCFC.
pub struct PciPio;
impl PioHandler for PciPio {
    fn handle(
        &self,
        _port: u16,
        direction: Direction,
        p: &dyn Probe,
        GenericVCpuState { vmcs, gprs, .. }: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        match direction {
            // On every out, make it no ops.
            Direction::Outb(_) | Direction::Outd(_) | Direction::Outw(_) => (),
            // On every in, just returns 0xffff.
            Direction::InbAl => {
                gprs.rax = 0xff;
            }
            Direction::InwAx | Direction::IndEax => {
                gprs.rax = 0xffff;
            }
            Direction::Inbm(gva) => unsafe {
                core::ptr::write_unaligned(
                    p.gva2hva(vmcs, gva).unwrap().into_usize() as *mut u8,
                    0xff,
                );
            },
            Direction::Inwm(gva) => unsafe {
                core::ptr::write_unaligned(
                    p.gva2hva(vmcs, gva).unwrap().into_usize() as *mut u16,
                    0xffff,
                );
            },
            Direction::Indm(gva) => unsafe {
                core::ptr::write_unaligned(
                    p.gva2hva(vmcs, gva).unwrap().into_usize() as *mut u32,
                    0xffffffff,
                );
            },
        };
        Ok(VmexitResult::Ok)
    }
}

pub struct CmosPio;
impl PioHandler for CmosPio {
    fn handle(
        &self,
        _port: u16,
        _direction: Direction,
        _p: &dyn Probe,
        _generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        // ignore.
        Ok(VmexitResult::Ok)
    }
}

pub struct ExitPio;
impl PioHandler for ExitPio {
    fn handle(
        &self,
        port: u16,
        direction: Direction,
        _p: &dyn Probe,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        if matches!(
            (port, direction),
            (0x604, Direction::Outd(0x2000)) | (0xB004, Direction::Outw(0x2000))
        ) {
            generic_vcpu_state.vm.upgrade().unwrap().exit(0);
            return Ok(VmexitResult::Exited(0));
        }
        Ok(VmexitResult::Ok)
    }
}
