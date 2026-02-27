//! Virtual machine configuration of project3-1.
use crate::{
    ept::{EptMappingError, ExtendedPageTable, Permission as EptPermission},
    mmio::PrinterDev,
    vmexit::mmio,
};
use keos::{
    addressing::{Kva, PAGE_MASK, Pa, Va},
    mm::{
        Page,
        page_table::{PageTableMappingError, PageTableRoot, Pde, Pdpe, Permission, Pml4e, Pte},
    },
};
use keos_project2::page_table::PageTable;
use kev::{
    VmError,
    vcpu::{
        Cr0, Cr4, GenericVCpuState, Rflags, VmexitResult,
        segmentation::{SEGMENT_TABLE, Segment},
        table::SystemTableRegister,
    },
    vm::Gpa,
    vm_control::*,
    vmcs::{ActiveVmcs, Field},
    vmexits::VmexitController,
};
use kev_project1::{hypercall::HypercallCtx, vmexit::hypercall};

/// The Vmbase with EPT.
#[derive(Default)]
pub struct EptVmBase {}

/// The Vmstate of EptVmBase.
pub struct SimpleEptVmState {
    code: &'static [u8],
}
impl SimpleEptVmState {
    pub fn new(code: &'static [u8]) -> Self {
        Self { code }
    }
}

/// Error for setup_vbsp.
#[derive(Debug)]
pub enum Error {
    EptError(EptMappingError),
    VmError(VmError),
    PageTableError(PageTableMappingError),
}

impl kev::vm::VmState for SimpleEptVmState {
    type Error = Error;
    type VcpuState = SimpleEptVcpuState;

    fn vcpu_state(&self) -> Self::VcpuState {
        let mut mmio_controller = mmio::Controller::new();
        mmio_controller.register(PrinterDev::default());
        SimpleEptVcpuState {
            ept: ExtendedPageTable::new(),
            page_table: PageTable(PageTableRoot::new_boxed()),
            vmexit_controller: (hypercall::Controller::new(HypercallCtx), (mmio_controller)),
        }
    }

    fn setup_vbsp(
        &self,
        vbsp_generic_state: &mut GenericVCpuState,
        vbsp_vcpu_state: &mut Self::VcpuState,
    ) -> Result<(), Self::Error> {
        // Prepare code pages.
        let mut pgs = keos::mm::ContigPages::new((self.code.len() + 0xfff) & !0xfff)
            .expect("Failed to allocate code page")
            .split();
        // Copy code to page
        let ((srcs, remainder), mut dsts) = (self.code.as_chunks::<0x1000>(), pgs.iter_mut());
        for src in srcs {
            let dst = dsts.next().unwrap();
            dst.inner_mut().copy_from_slice(src);
        }
        let dst = dsts.next().unwrap();
        dst.inner_mut()[..remainder.len()].copy_from_slice(remainder);

        // Map into page table.
        let mut base = pgs.first().unwrap().pa().into_usize();
        vbsp_generic_state
            .vmcs
            .write(Field::GuestRip, base as u64)
            .expect("Failed to write to guest rip.");

        for pg in pgs.into_iter() {
            vbsp_vcpu_state
                .page_table
                .map(
                    Va::new(base).unwrap(),
                    pg,
                    Permission::READ | Permission::EXECUTABLE,
                )
                .map_err(Error::PageTableError)?;
            base += 0x1000;
        }
        let gdt = SystemTableRegister::new(unsafe { (&raw const SEGMENT_TABLE).as_ref() }.unwrap());
        let base = gdt.address & !(PAGE_MASK as u64);
        for kva in (base..gdt.address + gdt.size as u64).step_by(0x1000) {
            let kva = Kva::new(kva as usize).unwrap();
            unsafe {
                vbsp_vcpu_state
                    .page_table
                    .do_map(kva.into_va(), kva.into_pa(), Permission::READ)
                    .map_err(Error::PageTableError)?;
            }
        }

        // Add mmio area.
        unsafe {
            vbsp_vcpu_state
                .page_table
                .do_map(
                    Va::new(0xcafe0000).unwrap(),
                    Pa::new(0xcafe0000).unwrap(),
                    Permission::READ | Permission::WRITE | Permission::EXECUTABLE,
                )
                .map_err(Error::PageTableError)?;
        }
        vbsp_vcpu_state
            .ept
            .map(
                Gpa::new(0xcafe0000).unwrap(),
                Page::new(),
                EptPermission::READ,
            )
            .map_err(Error::EptError)?;
        // gpa -> hpa mappings.
        unsafe {
            use core::slice::from_raw_parts;
            let table = vbsp_vcpu_state
                .add_gpa_mapping(vbsp_vcpu_state.page_table.pa().into_usize() & !PAGE_MASK)
                .map_err(Error::EptError)?;

            for pml4e in from_raw_parts(table, 512).iter().filter(|e| *e & 1 != 0) {
                let ntable = vbsp_vcpu_state
                    .add_gpa_mapping(Pml4e(*pml4e).pa().unwrap().into_usize())
                    .map_err(Error::EptError)?;
                for pdpe in from_raw_parts(ntable, 512).iter().filter(|e| *e & 1 != 0) {
                    let ntable = vbsp_vcpu_state
                        .add_gpa_mapping(Pdpe(*pdpe).pa().unwrap().into_usize())
                        .map_err(Error::EptError)?;
                    for pde in from_raw_parts(ntable, 512).iter().filter(|e| *e & 1 != 0) {
                        let ntable = vbsp_vcpu_state
                            .add_gpa_mapping(Pde(*pde).pa().unwrap().into_usize())
                            .map_err(Error::EptError)?;
                        for pte in from_raw_parts(ntable, 512).iter().filter(|e| *e & 1 != 0) {
                            let pa = Pte(*pte).pa().unwrap().into_usize();
                            if pa != 0xcafe0000 {
                                vbsp_vcpu_state
                                    .add_gpa_mapping(pa)
                                    .map_err(Error::EptError)?;
                            }
                        }
                    }
                }
            }
        }
        // Run a guest on 64bit mode directly.
        let vmcs = &vbsp_generic_state.vmcs;
        vmcs.write(
            Field::GuestCsSelector,
            Segment::KernelCode.into_selector().pack() as u64,
        )
        .map_err(Error::VmError)?;
        vmcs.write(
            Field::GuestEsSelector,
            Segment::KernelData.into_selector().pack() as u64,
        )
        .map_err(Error::VmError)?;
        vmcs.write(
            Field::GuestSsSelector,
            Segment::KernelData.into_selector().pack() as u64,
        )
        .map_err(Error::VmError)?;
        vmcs.write(
            Field::GuestDsSelector,
            Segment::KernelData.into_selector().pack() as u64,
        )
        .map_err(Error::VmError)?;
        vmcs.write(
            Field::GuestFsSelector,
            Segment::KernelData.into_selector().pack() as u64,
        )
        .map_err(Error::VmError)?;
        vmcs.write(
            Field::GuestGsSelector,
            Segment::KernelData.into_selector().pack() as u64,
        )
        .map_err(Error::VmError)?;
        vmcs.write(
            Field::GuestTrSelector,
            Segment::KernelData.into_selector().pack() as u64,
        )
        .map_err(Error::VmError)?;
        vmcs.write(Field::GuestLdtrSelector, 0)
            .map_err(Error::VmError)?;

        vmcs.write(Field::GuestCsBase, 0).map_err(Error::VmError)?;
        vmcs.write(Field::GuestEsBase, 0).map_err(Error::VmError)?;
        vmcs.write(Field::GuestSsBase, 0).map_err(Error::VmError)?;
        vmcs.write(Field::GuestDsBase, 0).map_err(Error::VmError)?;
        vmcs.write(Field::GuestFsBase, 0).map_err(Error::VmError)?;
        vmcs.write(Field::GuestGsBase, 0).map_err(Error::VmError)?;
        vmcs.write(Field::GuestLdtrBase, 0)
            .map_err(Error::VmError)?;
        vmcs.write(
            Field::GuestGdtrBase,
            SystemTableRegister::new(unsafe { (&raw const SEGMENT_TABLE).as_ref() }.unwrap())
                .address,
        )
        .map_err(Error::VmError)?;
        vmcs.write(Field::GuestIdtrBase, 0)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestTrBase, 0).map_err(Error::VmError)?;

        vmcs.write(Field::GuestCsLimit, u64::MAX)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestEsLimit, u64::MAX)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestSsLimit, u64::MAX)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestDsLimit, u64::MAX)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestFsLimit, u64::MAX)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestGsLimit, u64::MAX)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestLdtrLimit, 0xffff)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestGdtrLimit, 0xffff)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestIdtrLimit, 0xffff)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestTrLimit, 0x67)
            .map_err(Error::VmError)?;

        vmcs.write(Field::GuestCsAccessRights, 0xa09b)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestEsAccessRights, 0xc093)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestSsAccessRights, 0xc093)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestDsAccessRights, 0xc093)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestFsAccessRights, 0xc093)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestGsAccessRights, 0xc093)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestTrAccessRights, 0x8b)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestLdtrAccessRights, 0x10000)
            .map_err(Error::VmError)?;

        vmcs.write(Field::GuestActivityState, 0)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestInterruptibilityState, 0)
            .map_err(Error::VmError)?;

        vmcs.write(Field::GuestCr0, Cr0::current().bits())
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestCr4, Cr4::current().bits())
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestIa32Efer, 0x500)
            .map_err(Error::VmError)?;

        vmcs.write(Field::GuestLinkPointer, 0xffff_ffff)
            .map_err(Error::VmError)?;
        vmcs.write(Field::GuestLinkPointerHi, 0xffff_ffff)
            .map_err(Error::VmError)?;

        vmcs.write(Field::GuestDr7, 0).map_err(Error::VmError)?;
        vmcs.write(Field::GuestIa32Debugctl, 0)
            .map_err(Error::VmError)?;

        vmcs.write(Field::GuestRflags, Rflags::_1.bits())
            .map_err(Error::VmError)?;

        vmcs.write(
            Field::GuestCr3,
            vbsp_vcpu_state.page_table.pa().into_usize() as u64,
        )
        .map_err(Error::VmError)?;
        Ok(())
    }
}

/// The Vcpu state of NoEptVmState.
pub struct SimpleEptVcpuState {
    ept: ExtendedPageTable,
    page_table: PageTable,
    vmexit_controller: (hypercall::Controller<HypercallCtx>, mmio::Controller),
}

impl SimpleEptVcpuState {
    unsafe fn add_gpa_mapping(&mut self, pa: usize) -> Result<*const usize, EptMappingError> {
        unsafe {
            let hpa = Pa::new(pa).unwrap();
            self.ept
                .do_map(
                    Gpa::new(pa).unwrap(),
                    hpa,
                    EptPermission::READ | EptPermission::WRITE | EptPermission::EXECUTABLE,
                )
                .map(|_| hpa.into_kva().into_usize() as *const usize)
        }
    }
}

impl kev::vcpu::VCpuState for SimpleEptVcpuState {
    fn pinbase_ctls(&self) -> VmcsPinBasedVmexecCtl {
        VmcsPinBasedVmexecCtl::EXTERNAL_INTERRUPT_EXITING
    }
    fn procbase_ctls(&self) -> VmcsProcBasedVmexecCtl {
        VmcsProcBasedVmexecCtl::HLT_EXITING
            | VmcsProcBasedVmexecCtl::CR3LOADEXIT
            | VmcsProcBasedVmexecCtl::UNCONDIOEXIT
    }
    fn procbase_ctls2(&self) -> VmcsProcBasedSecondaryVmexecCtl {
        VmcsProcBasedSecondaryVmexecCtl::ENABLE_RDTSCP | VmcsProcBasedSecondaryVmexecCtl::ENABLE_EPT
    }
    fn entry_ctls(&self) -> VmcsEntryCtl {
        VmcsEntryCtl::IA32E_MODE_GUEST | VmcsEntryCtl::LOAD_IA32_EFER
    }
    fn exit_ctls(&self) -> VmcsExitCtl {
        VmcsExitCtl::HOST_ADDRESS_SPACE_SIZE | VmcsExitCtl::ACK_INTR_ON_EXIT
    }
    fn init_guest_state(&self, vmcs: &ActiveVmcs) -> Result<(), VmError> {
        vmcs.write(
            Field::Eptptr,
            self.ept.pa().into_usize() as u64 | (3 << 3) | 6,
        )?;
        Ok(())
    }

    fn handle_vmexit(
        &mut self,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        let exit_reason = generic_vcpu_state.vmcs.exit_reason()?;
        let Self {
            ept: mem,
            vmexit_controller,
            ..
        } = self;
        vmexit_controller.handle(exit_reason, mem, generic_vcpu_state)
    }
}
