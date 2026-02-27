//! Virtual machine control structure.
//!
//! Virtual machine control data structures (VMCS) are structures used by x86
//! during virtual machine execution. VMCS stores detailed information on how a
//! virtual machine will operate, including its CPU and memory, IO
//! configurations, interrupt handling, and other hardware settings.
//! The configuration for the VMCS in Project 2 is called "no_ept_vm." This
//! setup does not virtualize the memory, and instead directly maps guest
//! virtual addresses to the corresponding host virtual addresses.
//!
//! ## Tasks
//! In this project, you are required to configure the guest CR3 and RIP
//! registers. The guest page table is stored at the
//! vbsp_vcpu_state.mem.page_table. Setting the guest RIP register should be set
//! to the entry point of the guest virtual machine.
//!
//! If you implement the page table in project 1 incorrectly, even if you pass
//! the test cases, then the virtual machine will not operate properly.
use crate::{
    hypercall::HypercallCtx,
    vmexit::{cpuid, hypercall, msr, pio},
};
use keos::{
    addressing::{Kva, PAGE_MASK, Pa, Va},
    mm::{
        Page,
        page_table::{PageTableMappingError, PageTableRoot, Permission},
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
    vm::{Gpa, Gva},
    vm_control::*,
    vmcs::{ActiveVmcs, Field},
    vmexits::VmexitController,
};

/// The Vmstate of NoEptVmState.
#[derive(Default)]
pub struct NoEptVmState {
    code: &'static [u8],
}

/// Error for setup_vbsp.
#[derive(Debug)]
pub enum Error {
    /// Error that occurs when page table mapping failed.
    PageTableMappingError(PageTableMappingError),
    /// Error that occurs when vmwrite instruction failed.
    VmError(VmError),
}

impl NoEptVmState {
    /// Create a new instance of NoEptVmState
    pub fn new(code: &'static [u8]) -> Self {
        Self { code }
    }
}

impl kev::vm::VmState for NoEptVmState {
    type VcpuState = NoEptVcpuState;
    type Error = Error;

    fn vcpu_state(&self) -> Self::VcpuState {
        let (mut pio_ctl, hypercall_ctl, cpuid_ctl, mut msr_ctl) = (
            pio::Controller::new(),
            hypercall::Controller::new(HypercallCtx),
            cpuid::Controller::new(),
            msr::Controller::new(),
        );
        pio_ctl.register(3, crate::pio::PioHandlerDummy);
        pio_ctl.register(0x3f8, crate::pio::PioHandlerPrint);
        pio_ctl.register(0xbb, crate::pio::PioHandlerQueue::new());

        assert!(msr_ctl.insert(0xabc, crate::msr::StackMsr::new()));

        NoEptVcpuState {
            mem: NoEpt {
                page_table: PageTable(PageTableRoot::new_boxed()),
            },
            vmexit_controller: (pio_ctl, (hypercall_ctl, (cpuid_ctl, msr_ctl))),
        }
    }

    fn setup_vbsp(
        &self,
        vbsp_generic_state: &mut GenericVCpuState,
        vbsp_vcpu_state: &mut Self::VcpuState,
    ) -> Result<(), Self::Error> {
        const ENTRY: Va = Va::new(0x4000).unwrap();
        const WRITABLE: Va = Va::new(0x2000).unwrap();

        // allocate a page to be written by guest
        vbsp_vcpu_state
            .mem
            .page_table
            .map(
                WRITABLE,
                Page::new(),
                Permission::READ | Permission::WRITE | Permission::EXECUTABLE,
            )
            .map_err(Error::PageTableMappingError)?;

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
        let mut base = ENTRY;
        for pg in pgs.into_iter() {
            vbsp_vcpu_state
                .mem
                .page_table
                .map(base, pg, Permission::READ | Permission::EXECUTABLE)
                .map_err(Error::PageTableMappingError)?;
            base += 0x1000;
        }
        let gdt = SystemTableRegister::new(unsafe { (&raw const SEGMENT_TABLE).as_ref().unwrap() });
        let base = gdt.address & !(PAGE_MASK as u64);
        for kva in (base..gdt.address + gdt.size as u64).step_by(0x1000) {
            let kva = Kva::new(kva as usize).unwrap();
            let mut page = Page::new();
            page.inner_mut().copy_from_slice(unsafe {
                core::slice::from_raw_parts(kva.into_usize() as *const u8, 0x1000)
            });
            vbsp_vcpu_state
                .mem
                .page_table
                .map(kva.into_va(), page, Permission::READ)
                .map_err(Error::PageTableMappingError)?;
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
            SystemTableRegister::new(unsafe { (&raw const SEGMENT_TABLE).as_ref().unwrap() })
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
            vbsp_vcpu_state.mem.page_table.pa().into_usize() as u64,
        )
        .map_err(Error::VmError)?;

        vbsp_generic_state
            .vmcs
            .write(Field::GuestRip, ENTRY.into_usize() as u64)
            .map_err(Error::VmError)?;
        Ok(())
    }
}

struct NoEpt {
    page_table: PageTable,
}
impl kev::Probe for NoEpt {
    fn gpa2hpa(&self, _vmcs: &ActiveVmcs, _gpa: Gpa) -> Option<Pa> {
        // Because the term `gpa` is not existed when ept is not existed, we cannot be
        // reachable here.
        unreachable!()
    }
    fn gva2hpa(&self, _vmcs: &ActiveVmcs, gva: Gva) -> Option<Pa> {
        // In this type of VM, gva is same as hva.
        let (va, ofs) = {
            let gva = gva.into_usize();
            (Va::new(gva & !PAGE_MASK)?, gva & PAGE_MASK)
        };
        self.page_table
            .walk(va)
            .map(|pte| pte.pa())
            .ok()?
            .map(|pa| pa + ofs)
    }
}

/// The Vcpu state of NoEptVmState.
pub struct NoEptVcpuState {
    mem: NoEpt,
    vmexit_controller: (
        pio::Controller,
        (
            hypercall::Controller<crate::hypercall::HypercallCtx>,
            (cpuid::Controller, msr::Controller),
        ),
    ),
}

impl kev::vcpu::VCpuState for NoEptVcpuState {
    fn pinbase_ctls(&self) -> VmcsPinBasedVmexecCtl {
        VmcsPinBasedVmexecCtl::EXTERNAL_INTERRUPT_EXITING
    }
    fn procbase_ctls(&self) -> VmcsProcBasedVmexecCtl {
        VmcsProcBasedVmexecCtl::HLT_EXITING
            | VmcsProcBasedVmexecCtl::CR3LOADEXIT
            | VmcsProcBasedVmexecCtl::UNCONDIOEXIT
    }
    fn procbase_ctls2(&self) -> VmcsProcBasedSecondaryVmexecCtl {
        VmcsProcBasedSecondaryVmexecCtl::ENABLE_RDTSCP
    }
    fn entry_ctls(&self) -> VmcsEntryCtl {
        VmcsEntryCtl::IA32E_MODE_GUEST | VmcsEntryCtl::LOAD_IA32_EFER
    }
    fn exit_ctls(&self) -> VmcsExitCtl {
        VmcsExitCtl::HOST_ADDRESS_SPACE_SIZE | VmcsExitCtl::ACK_INTR_ON_EXIT
    }
    fn init_guest_state(&self, _vmcs: &ActiveVmcs) -> Result<(), VmError> {
        Ok(())
    }

    fn handle_vmexit(
        &mut self,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        // Hint: Use Vmcs.
        let exit_reason = generic_vcpu_state
            .vmcs
            .exit_reason()
            .expect("unexpected vmexit.");
        let Self {
            mem,
            vmexit_controller,
        } = self;
        vmexit_controller.handle(exit_reason, mem, generic_vcpu_state)
    }
}
