//! Virtual CPU implementation.
use crate::{
    VmError,
    vm::{Vm, VmOps, VmState},
    vm_control::*,
    vmcs::{ActiveVmcs, BasicExitReason, ExternalIntInfo, Field, Vmcs},
};
pub use abyss::{interrupt::GeneralPurposeRegisters, x86_64::*};
use alloc::sync::Weak;
use core::{
    arch::naked_asm,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
use interrupt::IDT;
use intrinsics::read_cr3;
use keos::sync::SpinLock;
use msr::Msr;
use segmentation::{SEGMENT_TABLE, Segment, SegmentTable};
use table::SystemTableRegister;

#[unsafe(naked)]
#[allow(binary_asm_labels)]
unsafe extern "C" fn vmlaunch_resume(
    _gp: &mut GeneralPurposeRegisters,
    _launched: &mut bool,
) -> i8 {
    naked_asm!(
        "push rbx",
        "push rbp",
        "push rdi",
        "clc",
        "mov rax, 0x6c14", // HostRsp.
        "vmwrite rax, rsp",
        "setna al",
        // If failed, return.
        "cmp al, 0",
        "je 1f",
        "pop rdi",
        "pop rbp",
        "pop rbx",
        "mov al, 1",
        "ret",
        // do vmlaunch.
        "1: ",
        "cmp rsi, 1",
        "mov rax, [rdi + 0x78]",
        "mov cr2, rax",
        "mov rax, [rdi + 0x70]",
        "mov rbx, [rdi + 0x68]",
        "mov rcx, [rdi + 0x60]",
        "mov rdx, [rdi + 0x58]",
        "mov rbp, [rdi + 0x50]",
        "mov rsi, [rdi + 0x40]",
        "mov r8, [rdi + 0x38]",
        "mov r9, [rdi + 0x30]",
        "mov r10, [rdi + 0x28]",
        "mov r11, [rdi + 0x20]",
        "mov r12, [rdi + 0x18]",
        "mov r13, [rdi + 0x10]",
        "mov r14, [rdi + 0x8]",
        "mov r15, [rdi]",
        "mov rdi, [rdi + 0x48]",
        "jne 2f",
        "vmresume",
        "jmp 3f",
        "2:",
        "vmlaunch",
        "3:",
        "pop rdi",
        "pop rbp",
        "pop rbx",
        "mov al, 2",
        "ret",
    )
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn vmexit() {
    naked_asm!(
        "sub rsp, 8",
        "mov [rsp], rdi",
        "mov rdi, [rsp + 8]",
        "mov [rdi + 0x70], rax",
        "mov rax, cr2",
        "mov [rdi + 0x78], rax",
        "mov [rdi + 0x68], rbx",
        "mov [rdi + 0x60], rcx",
        "mov [rdi + 0x58], rdx",
        "mov [rdi + 0x50], rbp",
        "mov [rdi + 0x40], rsi",
        "mov [rdi + 0x38], r8",
        "mov [rdi + 0x30], r9",
        "mov [rdi + 0x28], r10",
        "mov [rdi + 0x20], r11",
        "mov [rdi + 0x18], r12",
        "mov [rdi + 0x10], r13",
        "mov [rdi + 0x8], r14",
        "mov [rdi], r15",
        "mov rax, [rsp]",
        "mov [rdi + 0x48], rax",
        "add rsp, 8",
        "mov rsi, 1",
        "xor rax, rax",
        "pop rdi",
        "pop rbp",
        "pop rbx",
        "ret",
    )
}

/// Per-vcpu private state.
pub trait VCpuState
where
    Self: Sync + Send + 'static,
{
    /// Get enabled pin-based control fields.
    fn pinbase_ctls(&self) -> VmcsPinBasedVmexecCtl;
    /// Get enabled proc-based control fields.
    fn procbase_ctls(&self) -> VmcsProcBasedVmexecCtl;
    /// Get enabled proc-based secondary control fields.
    fn procbase_ctls2(&self) -> VmcsProcBasedSecondaryVmexecCtl;
    /// Get enabled exit control fields.
    fn exit_ctls(&self) -> VmcsExitCtl;
    /// Get enabled entry control fields.
    fn entry_ctls(&self) -> VmcsEntryCtl;
    /// Initialize the guest state.
    fn init_guest_state(&self, vmcs: &ActiveVmcs) -> Result<(), VmError>;
    /// Handle the vmexit on this vcpu.
    fn handle_vmexit(
        &mut self,
        genenric_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError>;
}

/// A visible state for VCpu.
pub struct GenericVCpuState<'a> {
    /// The activated vmcs.
    pub vmcs: ActiveVmcs,
    /// general purpose register of the vcpu.
    pub gprs: &'a mut GeneralPurposeRegisters,
    /// Weak reference of the vm.
    pub vm: Weak<dyn VmOps>,
    // smp id of this vcpu.
    id: usize,
    // Pending interrupts.
    pending_interrupts: &'a [AtomicU64; 4],
}

impl<'a> GenericVCpuState<'a> {
    /// Get smp id of this vcpu.
    #[inline]
    pub fn id(&self) -> usize {
        self.id
    }

    /// Inject the interrupt `vec` into the `active_vmcs`.
    pub fn inject_interrupt(&self, vec: u8) {
        // Inject interrupt to the interrupt window
        let (index, ofs) = (vec / 64, vec & 63);
        self.pending_interrupts[index as usize].store(1 << ofs, Ordering::SeqCst);
    }
}

/// Virtual cpu.
#[repr(C, align(4096))]
pub struct VCpu<S: VmState + 'static> {
    // This must be the first field of the VCpu.
    vmcs: Vmcs,
    // general purpose register of the vcpu.
    gprs: GeneralPurposeRegisters,
    /// Indicate whether this vcpu is launched after vmclear.
    launched: bool,
    /// vcpu id.
    pub vcpu_id: usize,
    /// The state of VCpu.
    state: S::VcpuState,
    /// Vm that owned this VCpu.
    vm: Weak<Vm<S>>,
    /// pending interrupt bitmask
    pending_interrupts: [AtomicU64; 4],
}

impl<S: VmState + 'static> VCpu<S> {
    pub(crate) fn new(vcpu_id: usize, state: S::VcpuState, vm: Weak<Vm<S>>) -> Self {
        Self {
            vmcs: Vmcs::new(),
            gprs: GeneralPurposeRegisters::default(),
            launched: false,
            vcpu_id,
            state,
            vm,
            pending_interrupts: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
        }
    }

    pub(crate) fn unpack_activate(&mut self) -> Result<Activated<'_, S>, VmError> {
        let Self {
            vmcs,
            gprs,
            vcpu_id,
            state,
            launched,
            vm,
            pending_interrupts,
        } = self;
        Ok(Activated {
            generic_state: GenericVCpuState {
                vmcs: Vmcs::activate(vmcs)?,
                gprs,
                id: *vcpu_id,
                vm: vm.clone(),
                pending_interrupts,
            },
            vcpu_state: state,
            launched,
            vmcs,
        })
    }
}

/// VCpuOps
pub trait VCpuOps
where
    Self: Send + Sync,
{
    /// Inject interrupt to the VCpu with vec.
    fn inject_interrupt(&self, vec: u8);
}

impl<S: VmState + 'static> VCpuOps for SpinLock<VCpu<S>> {
    fn inject_interrupt(&self, vec: u8) {
        let (index, ofs) = (vec / 64, vec & 63);
        let guard = self.lock();
        guard.pending_interrupts[index as usize].store(1 << ofs, Ordering::SeqCst);
        guard.unlock();
    }
}

pub(crate) struct Activated<'a, S: VmState + 'static> {
    pub(crate) generic_state: GenericVCpuState<'a>,
    pub(crate) vcpu_state: &'a mut S::VcpuState,
    vmcs: &'a mut Vmcs,
    launched: &'a mut bool,
}

impl<'a, S: VmState + 'static> Activated<'a, S> {
    pub(crate) unsafe fn init_vcpu(&mut self, exception_bitmap: u32) -> Result<(), VmError> {
        unsafe {
            let Self {
                generic_state: GenericVCpuState { vmcs, .. },
                vcpu_state,
                ..
            } = self;
            // 26.2.1.1 VM-Execution Control Fields
            {
                // Reserved bits in the pin-based VM-execution controls must be set properly.
                // Software may consult the VMX capability MSRs to determine the
                // proper settings (see Appendix A.3.1).
                {
                    let pinbase_ctls = Msr::<IA32_VMX_PINBASED_CTLS>::read();
                    let (supported, mut enabled) = (
                        VmcsPinBasedVmexecCtl::from_bits_unchecked((pinbase_ctls >> 32) as u32),
                        VmcsPinBasedVmexecCtl::from_bits_unchecked(
                            pinbase_ctls as u32 & !(VmcsPinBasedVmexecCtl::all().bits()),
                        ),
                    );
                    // enable the guest external interrupt exit
                    enabled |= vcpu_state.pinbase_ctls();
                    vmcs.write(
                        Field::PinBasedExecControls,
                        (enabled & supported).bits() as u64,
                    )?;
                }
                // Reserved bits in the primary processor-based VM-execution controls must be
                // set properly. Software may consult the VMX capability MSRs to
                // determine the proper settings (see Appendix A.3.2).
                {
                    let procbase_ctls = Msr::<IA32_VMX_PROC_BASED_CTLS>::read();
                    let (supported, mut enabled) = (
                        VmcsProcBasedVmexecCtl::from_bits_unchecked((procbase_ctls >> 32) as u32),
                        VmcsProcBasedVmexecCtl::from_bits_unchecked(
                            procbase_ctls as u32 & !(VmcsProcBasedVmexecCtl::all().bits()),
                        ),
                    );
                    // Make sure there are secondary controls.
                    assert!(supported.contains(VmcsProcBasedVmexecCtl::ACTIVATE_SECONDARY_CTL));
                    enabled |= VmcsProcBasedVmexecCtl::ACTIVATE_SECONDARY_CTL;
                    enabled |= vcpu_state.procbase_ctls();
                    vmcs.write(
                        Field::ProcessorBasedVmexecControls,
                        (enabled & supported).bits() as u64,
                    )?;
                }
                // If the “activate secondary controls” primary processor-based VM-execution
                // control is 1, reserved bits in the secondary processor-based
                // VM-execution controls must be cleared. Software may consult the VMX
                // capability MSRs to determine which bits are reserved (see
                // Appendix A.3.3).
                {
                    let procbase_ctls2 = Msr::<IA32_VMX_PROC_BASED_CTLS2>::read();
                    let (supported, mut enabled) = (
                        VmcsProcBasedSecondaryVmexecCtl::from_bits_unchecked(
                            (procbase_ctls2 >> 32) as u32,
                        ),
                        VmcsProcBasedSecondaryVmexecCtl::from_bits_unchecked(
                            procbase_ctls2 as u32
                                & !(VmcsProcBasedSecondaryVmexecCtl::all().bits()),
                        ),
                    );
                    enabled |= vcpu_state.procbase_ctls2();
                    vmcs.write(
                        Field::SecondaryVmexecControls,
                        (enabled & supported).bits() as u64,
                    )?;
                }
                // 26.2.1.2 VM-Exit Control Fields
                {
                    // Reserved bits in the primary VM-exit controls must be set properly.
                    // Software may consult the VMX capability MSRs to determine the proper settings
                    // (see Appendix A.4.1).
                    let exit_ctls = Msr::<IA32_VMX_EXIT_CTLS>::read();
                    let (supported, mut enabled) = (
                        VmcsExitCtl::from_bits_unchecked((exit_ctls >> 32) as u32),
                        VmcsExitCtl::from_bits_unchecked(exit_ctls as u32),
                    );
                    enabled |= vcpu_state.exit_ctls();
                    vmcs.write(Field::VmexitControls, (enabled & supported).bits() as u64)?;
                }
                // 26.2.1.3 VM-Entry Control Fields
                {
                    // Reserved bits in the VM-entry controls must be set properly.
                    // Software may consult the VMX capability MSRs to determine the proper settings
                    // (see Appendix A.5).
                    let entry_ctls = Msr::<IA32_VMX_ENTRY_CTLS>::read();
                    let (supported, mut enabled) = (
                        VmcsEntryCtl::from_bits_unchecked((entry_ctls >> 32) as u32),
                        VmcsEntryCtl::from_bits_unchecked(entry_ctls as u32),
                    );
                    enabled |= vcpu_state.entry_ctls();
                    vmcs.write(Field::VmentryControls, (supported & enabled).bits() as u64)?;
                }
                vmcs.write(Field::ExceptionBitmap, exception_bitmap as u64)?;
            }
            // 26.2.2 Checks on Host Control Registers, MSRs, and SSP
            // 26.2.3 Checks on Host Segment and Descriptor-Table Registers
            // 26.2.4 Checks Related to Address-Space Size
            {
                // The CR0 field must not set any bit to a value not supported in VMX operation
                // (see Section 23.8).
                vmcs.write(Field::HostCr0, Cr0::current().bits())?;
                // The CR4 field must not set any bit to a value not supported in VMX operation
                // (see Section 23.8).
                vmcs.write(Field::HostCr4, Cr4::current().bits())?;
                vmcs.write(Field::HostCr3, read_cr3() as u64)?;

                // Load segments
                vmcs.write(
                    Field::HostEsSelector,
                    Segment::KernelData.into_selector().pack() as u64,
                )?;
                vmcs.write(
                    Field::HostSsSelector,
                    Segment::KernelData.into_selector().pack() as u64,
                )?;
                vmcs.write(
                    Field::HostDsSelector,
                    Segment::KernelData.into_selector().pack() as u64,
                )?;
                vmcs.write(
                    Field::HostFsSelector,
                    Segment::KernelData.into_selector().pack() as u64,
                )?;
                vmcs.write(
                    Field::HostGsSelector,
                    Segment::KernelData.into_selector().pack() as u64,
                )?;
                vmcs.write(
                    Field::HostCsSelector,
                    Segment::KernelCode.into_selector().pack() as u64,
                )?;
                vmcs.write(
                    Field::HostTrSelector,
                    Segment::Tss.into_selector().pack() as u64,
                )?;

                // Load gdt, Idt
                vmcs.write(
                    Field::HostGdtrBase,
                    SystemTableRegister::new({ &raw const SEGMENT_TABLE }.as_ref().unwrap())
                        .address,
                )?;
                vmcs.write(
                    Field::HostIdtrBase,
                    SystemTableRegister::new({ &raw const IDT }.as_ref().unwrap()).address,
                )?;

                // Load gs, fs, tr
                vmcs.write(Field::HostFsBase, 0)?;
                vmcs.write(
                    Field::HostGsBase,
                    &mut kernel_gs::KERNEL_GS_BASES[intrinsics::cpuid()] as *mut kernel_gs::KernelGS
                        as u64,
                )?;
                let tss = SegmentTable::current_tss();
                vmcs.write(Field::HostTrBase, tss as *mut _ as usize as u64)?;

                // Vmexit location
                vmcs.write(Field::HostRip, vmexit as *const () as usize as u64)?;
            }
            vcpu_state.init_guest_state(vmcs)
        }
    }

    pub fn vcpu_loop(&mut self, have_kicked: &AtomicBool) -> Result<VmexitResult, VmError> {
        assert_eq!(
            abyss::interrupt::InterruptState::current(),
            abyss::interrupt::InterruptState::Off
        );
        let Self {
            generic_state,
            vcpu_state,
            launched,
            ..
        } = self;
        unsafe {
            loop {
                // CHAPTER 26. VM ENTRIES
                //
                // Each VM entry performs the following steps in the order indicated:
                // 1. Basic checks are performed to ensure that VM entry can commence (Section
                //    26.1).
                // 2. The control and host-state areas of the VMCS are checked to ensure that
                //    they are proper for supporting VMX
                // non-root operation and that the VMCS is correctly configured to support the
                // next VM exit (Section 26.2).
                // 3. The following may be performed in parallel or in any order (Section 26.3):
                // - The guest-state area of the VMCS is checked to ensure that, after the VM
                //   entry completes, the state of the
                // logical processor is consistent with IA-32 and Intel 64 architectures.
                // - Processor state is loaded from the guest-state area and based on controls
                //   in the VMCS.
                // - Address-range monitoring is cleared.
                // 4. MSRs are loaded from the VM-entry MSR-load area (Section 26.4).
                // 5. If VMLAUNCH is being executed, the launch state of the VMCS is set to
                //    “launched.”
                // 6. If the “Intel PT uses guest physical addresses” VM-execution control is 1,
                //    trace-address pre-translation (TAPT)
                // may occur (see Section 25.5.4 and Section 26.5).
                // 7. An event may be injected in the guest context (Section 26.6).
                //
                // Steps 1–4 above perform checks that may cause VM entry to fail. Such failures
                // occur in one of the following three ways:
                // - Some of the checks in Section 26.1 may generate ordinary faults (for
                //   example, an invalid-opcode exception).
                // Such faults are delivered normally.
                // - Some of the checks in Section 26.1 and all the checks in Section 26.2 cause
                //   control to pass to the instruction
                // following the VM-entry instruction. The failure is indicated by setting
                // RFLAGS.ZF1 (if there is a current VMCS) or RFLAGS.CF (if
                // there is no current VMCS). If there is a current VMCS, an error number
                // indicating the cause of the failure is stored in the
                // VM-instruction error field. See Chapter 30 for the error numbers.

                // Inject pending interrupt if exists.
                for (index, intr_bitmap) in generic_state.pending_interrupts.iter().enumerate() {
                    let v = intr_bitmap.load(Ordering::SeqCst);
                    if v != 0 {
                        let guest_rflags = Rflags::from_bits_truncate(
                            generic_state
                                .vmcs
                                .read(Field::GuestRflags)
                                .expect("Failed to read guest rflags."),
                        );
                        if guest_rflags.contains(Rflags::IF) {
                            let ofs = v.trailing_zeros() as usize;
                            intr_bitmap.fetch_and(!(1 << ofs), Ordering::SeqCst);
                            let vec = (index * 64 + ofs) as u64;
                            generic_state
                                .vmcs
                                .write(Field::VmentryInterruptionInfo, vec | (1 << 31))
                                .expect("Failed to set VmentryInterruptionInfo.");
                        } else {
                            // We required to wait until Rflags::IF is set. Trap immediatly when it
                            // becomes 1.
                            let proc_based_ctls = VmcsProcBasedVmexecCtl::from_bits_unchecked(
                                generic_state
                                    .vmcs
                                    .read(Field::ProcessorBasedVmexecControls)
                                    .expect("Failed to read vmcs field")
                                    as u32,
                            ) | VmcsProcBasedVmexecCtl::INTRWINEXIT;
                            generic_state
                                .vmcs
                                .write(
                                    Field::ProcessorBasedVmexecControls,
                                    proc_based_ctls.bits() as u64,
                                )
                                .expect("Failed to update ProcessorBasedVmexecControls.");
                        }
                        break;
                    }
                }

                // Check whether this vcpu is kicked.
                if have_kicked.load(Ordering::SeqCst) {
                    return Ok(VmexitResult::Kicked);
                }

                generic_state.vmcs.write(
                    Field::HostGsBase,
                    &mut kernel_gs::KERNEL_GS_BASES[intrinsics::cpuid()] as *mut kernel_gs::KernelGS
                        as u64,
                )?;
                let mut result: i64;
                {
                    let mut vcpu_state: i64 = **launched as i64;
                    core::arch::asm!(
                        "call {}",
                        sym vmlaunch_resume,
                        in("rdi") generic_state.gprs,
                        inout("rsi") vcpu_state,
                        lateout("rax") result,
                        lateout("rcx") _,
                        lateout("rdx") _,
                        lateout("rdi") _,
                        lateout("r8") _,
                        lateout("r9") _,
                        lateout("r10") _,
                        lateout("r11") _,
                        lateout("r12") _,
                        lateout("r13") _,
                        lateout("r14") _,
                        lateout("r15") _
                    );
                    **launched = vcpu_state != 0;
                }

                match result {
                    0 => {
                        let rip = generic_state.vmcs.read(Field::GuestRip)?;
                        if let Err(err) = match generic_state.vmcs.exit_reason()?.get_basic_reason()
                        {
                            BasicExitReason::ExternalInt(Some(ExternalIntInfo {
                                host_int,
                                ..
                            })) => {
                                return Ok(VmexitResult::ExtInt(*host_int));
                            }
                            BasicExitReason::InterruptWindow => {
                                let proc_based_ctls = VmcsProcBasedVmexecCtl::from_bits_unchecked(
                                    generic_state
                                        .vmcs
                                        .read(Field::ProcessorBasedVmexecControls)
                                        .expect("Failed to read vmcs field")
                                        as u32
                                        & !VmcsProcBasedVmexecCtl::INTRWINEXIT.bits(),
                                );
                                generic_state
                                    .vmcs
                                    .write(
                                        Field::ProcessorBasedVmexecControls,
                                        proc_based_ctls.bits() as u64,
                                    )
                                    .expect("Failed to update ProcessorBasedVmexecControls.");
                                Ok(())
                            }
                            _ => match vcpu_state.handle_vmexit(generic_state) {
                                Ok(VmexitResult::Ok) => Ok(()),
                                r => return r,
                            },
                        } {
                            println!("err {:?} rip: {:x}", err, rip);
                            generic_state.vmcs.dump();
                            return Err(err);
                        }
                    }
                    1 | 2 => return Err(VmError::VmxOperationError(Vmcs::instruction_error())),
                    _ => unreachable!(),
                }
            }
        }
    }
}

impl<'a, S: VmState> Drop for Activated<'a, S> {
    fn drop(&mut self) {
        *self.launched = false;
        self.vmcs.clear().unwrap();
    }
}

/// Possible result of the Vmexit.
#[derive(Debug)]
pub enum VmexitResult {
    /// VCpu can be continued.
    Ok,
    /// VCpu is exited.
    Exited(i32),
    /// External Interrupt is come.
    ///
    /// This is for internal-control uses.
    ExtInt(u8),
    /// VCpu is kicked.
    ///
    /// This is for internal-control uses.
    Kicked,
}
