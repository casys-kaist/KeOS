//! X2apic msrs.
use kev::{Probe, VmError, vcpu::GenericVCpuState};
use kev_project1::vmexit::msr;

pub struct X2Apic {}

impl X2Apic {
    pub fn attach(ctl: &mut msr::Controller) {
        // APIC_BASE MSR.
        assert!(ctl.insert(0x1B, X2Apic {}));
        // TP.
        assert!(ctl.insert(0x808, X2Apic {}));
        // eoi
        assert!(ctl.insert(0x80b, X2Apic {}));
        // ipi
        assert!(ctl.insert(0x830, X2Apic {}));
        // timer
        assert!(ctl.insert(0x832, X2Apic {}));
        // Susprious interrupt vector.
        assert!(ctl.insert(0x80F, X2Apic {}));
        // lint0
        assert!(ctl.insert(0x835, X2Apic {}));
        // lint1
        assert!(ctl.insert(0x836, X2Apic {}));

        // tsc_deadline
        assert!(ctl.insert(0x6e0, X2Apic {}));
    }
}

impl msr::Msr for X2Apic {
    fn rdmsr(
        &self,
        index: u32,
        _p: &dyn Probe,
        _generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<u64, VmError> {
        match index {
            0x1b | 0x808 | 0x80b | 0x830 | 0x80f | 0x835 | 0x836 | 0x832 | 0x6e0 => Ok(0),
            _ => todo!("{index:x}"),
        }
    }

    fn wrmsr(
        &mut self,
        index: u32,
        _value: u64,
        _p: &dyn Probe,
        _generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<(), VmError> {
        match index {
            0x1b | 0x808 | 0x80b | 0x830 | 0x80f | 0x835 | 0x836 | 0x832 | 0x6e0 => (),
            _ => todo!("{index:x}"),
        }
        Ok(())
    }
}
