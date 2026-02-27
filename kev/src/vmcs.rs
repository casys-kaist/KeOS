//! Virtual-Machine Control State (VMCS) related apis.
use crate::{
    Probe,
    vm::{Gpa, Gva},
    {VmError, vm_control::*},
};
use abyss::{
    addressing::{Kva, Pa},
    x86_64::msr::Msr,
};
use alloc::{boxed::Box, format};
use core::arch::asm;
use iced_x86::{Decoder, DecoderOptions, Instruction};

/// Virtual Machine Control State.
///
/// This holds virtual machine specific information.
///
/// ## Details
/// See Intel® 64 and IA-32 Architectures Software Developer’s Manual, 24.2
/// FORTMAT OF VMCS.
#[repr(align(4096))]
pub struct Vmcs {
    /// Bits 30:0: VMCS revision identifier
    /// Bit 31: shadow-VMCS indicator (see Section 24.10)
    rev_id: u32,
    /// VMX-abort indicator
    indicator: u32,
    /// VMCS data (implementation-specific format)
    _data: [u8; 0x1000 - 4],
}

/// Possible errors for vm-related instructions.
///
/// # Details
/// See Intel® 64 and IA-32 Architectures Software Developer’s Manual, Table
/// 30-1. Vm-Instruction Error Numbers.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub enum InstructionError {
    /// VMCALL executed in VMX root operation
    VmcallInVmxRoot,
    /// VMCLEAR with invalid physical address
    VmclearWithInvAddr,
    /// VMCLEAR with VMXON pointer
    VmclearWithVmxon,
    /// VMLAUNCH with non-clear VMCS
    VmresumeWithNonclearVmcs,
    /// VMRESUME with non-launched VMCS
    VmresumeWithNonlaunchedVmcs,
    /// VMRESUME after VMXOFF
    VmresumeAfterVmxoff,
    /// VM entry with invalid control field(s)
    InvalidCs,
    /// VM entry with invalid host-state field(s)2
    InvalidHostState,
    /// VMPTRLD with invalid physical address
    VmPtrLdWithInvAddr,
    /// VMPTRLD with VMXON pointer
    VmPtrLdWithVmxOn,
    /// VMPTRLD with incorrect VMCS revision identifier
    VmPtrLdWithIncorrectRevId,
    /// VMREAD/VMWRITE from/to unsupported VMCS component
    UnsupportedVmcsField,
    /// VMWRITE to read-only VMCS component
    WriteToRoField,
    /// VMXON executed in VMX root operation
    VmxonInVmxRoot,
    /// VM entry with invalid executive-VMCS pointer2
    VmEntryWithInvalidExecVmcs2,
    /// VM entry with non-launched executive VMCS2
    VmEntryWithNonlaucnhedExecVmcs2,
    /// VM entry with executive-VMCS pointer not VMXON pointer (when attempting
    /// to deactivate the dual-monitor treatment of SMIs and SMM)2
    VmEntryWithExecVmcs,
    /// VMCALL with non-clear VMCS (when attempting to activate the dual-monitor
    /// treatment of SMIs and SMMk)
    VmcallWithNonclearVmcs,
    /// VMCALL with invalid VM-exit control fields
    VmcallWithInvVmexitCs,
    /// VMCALL with incorrect MSEG revision identifier (when attempting to
    /// activate the dual-monitor treatment of SMIs and SMM)
    VmcallWithIncorrectMsegRevId,
    /// VMXOFF under dual-monitor treatment of SMIs and SMM
    VmxoffUnderDualMontiroTreatment,
    /// VMCALL with invalid SMM-monitor features (when attempting to activate
    /// the dual-monitor treatment of SMIs and SMM)
    VmcallWithInvSmmMonitor,
    /// VM entry with invalid VM-execution control fields in executive VMCS
    /// (when attempting to return from SMM)2,3
    VmentryWithInvVmExecCs,
    /// VM entry with events blocked by MOV SS.
    VmentryWithEventBlockedByMovss,
    /// Invalid operand to INVEPT/INVVPID.
    InvalidOperandToInveptInvvpid,
    /// Unknown error.
    Unknown,
}

/// Vmcs field.
#[allow(missing_docs)]
#[repr(i32)]
#[derive(Debug)]
pub enum Field {
    // 16bit fields
    Vpid = 0x00000000,
    PostedInterruptVector = 0x00000002,
    EptpIndex = 0x00000004,
    GuestEsSelector = 0x00000800,
    GuestCsSelector = 0x00000802,
    GuestSsSelector = 0x00000804,
    GuestDsSelector = 0x00000806,
    GuestFsSelector = 0x00000808,
    GuestGsSelector = 0x0000080A,
    GuestLdtrSelector = 0x0000080C,
    GuestTrSelector = 0x0000080E,
    GuestInterruptStatus = 0x00000810,
    HostEsSelector = 0x00000C00,
    HostCsSelector = 0x00000C02,
    HostSsSelector = 0x00000C04,
    HostDsSelector = 0x00000C06,
    HostFsSelector = 0x00000C08,
    HostGsSelector = 0x00000C0A,
    HostTrSelector = 0x00000C0C,
    // 64bit fields
    IoBitmapA = 0x00002000,
    IoBitmapAHi = 0x00002001,
    IoBitmapB = 0x00002002,
    IoBitmapBHi = 0x00002003,
    MsrBitmaps = 0x00002004,
    MsrBitmapsHi = 0x00002005,
    VmexitMsrStoreAddr = 0x00002006,
    VmexitMsrStoreAddrHi = 0x00002007,
    VmexitMsrLoadAddr = 0x00002008,
    VmexitMsrLoadAddrHi = 0x00002009,
    VmentryMsrLoadAddr = 0x0000200A,
    VmentryMsrLoadAddrHi = 0x0000200B,
    ExecutiveVmcsPtr = 0x0000200C,
    ExecutiveVmcsPtrHi = 0x0000200D,
    TscOffset = 0x00002010,
    TscOffsetHi = 0x00002011,
    VirtualApicPageAddr = 0x00002012,
    VirtualApicPageAddrHi = 0x00002013,
    ApicAccessAddr = 0x00002014,
    ApicAccessAddrHi = 0x00002015,
    PostedInterruptDescAddr = 0x00002016,
    PostedInterruptDescAddrHi = 0x00002017,
    VmfuncCtrls = 0x00002018,
    VmfuncCtrlsHi = 0x00002019,
    Eptptr = 0x0000201A,
    EptptrHi = 0x0000201B,
    EoiExitBitmap0 = 0x0000201C,
    EoiExitBitmap0Hi = 0x0000201D,
    EoiExitBitmap1 = 0x0000201E,
    EoiExitBitmap1Hi = 0x0000201F,
    EoiExitBitmap2 = 0x00002020,
    EoiExitBitmap2Hi = 0x00002021,
    EoiExitBitmap3 = 0x00002022,
    EoiExitBitmap3Hi = 0x00002023,
    EptpListAddress = 0x00002024,
    EptpListAddressHi = 0x00002025,
    VmreadBitmapAddr = 0x00002026,
    VmreadBitmapAddrHi = 0x00002027,
    VmwriteBitmapAddr = 0x00002028,
    VmwriteBitmapAddrHi = 0x00002029,
    VeExceptionInfoAddr = 0x0000202A,
    VeExceptionInfoAddrHi = 0x0000202B,
    GuestPhysicalAddr = 0x00002400,
    GuestPhysicalAddrHi = 0x00002401,
    GuestLinkPointer = 0x00002800,
    GuestLinkPointerHi = 0x00002801,
    GuestIa32Debugctl = 0x00002802,
    GuestIa32DebugctlHi = 0x00002803,
    GuestIa32Pat = 0x00002804,
    GuestIa32PatHi = 0x00002805,
    GuestIa32Efer = 0x00002806,
    GuestIa32EferHi = 0x00002807,
    GuestIa32PerfGlobalCtrl = 0x00002808,
    GuestIa32PerfGlobalCtrlHi = 0x00002809,
    GuestIa32Pdpte0 = 0x0000280A,
    GuestIa32Pdpte0Hi = 0x0000280B,
    GuestIa32Pdpte1 = 0x0000280C,
    GuestIa32Pdpte1Hi = 0x0000280D,
    GuestIa32Pdpte2 = 0x0000280E,
    GuestIa32Pdpte2Hi = 0x0000280F,
    GuestIa32Pdpte3 = 0x00002810,
    GuestIa32Pdpte3Hi = 0x00002811,
    HostIa32Pat = 0x00002C00,
    HostIa32PatHi = 0x00002C01,
    HostIa32Efer = 0x00002C02,
    HostIa32EferHi = 0x00002C03,
    HostIa32PerfGlobalCtrl = 0x00002C04,
    HostIa32PerfGlobalCtrlHi = 0x00002C05,
    // 32bit fields
    PinBasedExecControls = 0x00004000,
    ProcessorBasedVmexecControls = 0x00004002,
    ExceptionBitmap = 0x00004004,
    PageFaultErrCodeMask = 0x00004006,
    PageFaultErrCodeMatch = 0x00004008,
    Cr3TargetCount = 0x0000400A,
    VmexitControls = 0x0000400C,
    VmexitMsrStoreCount = 0x0000400E,
    VmexitMsrLoadCount = 0x00004010,
    VmentryControls = 0x00004012,
    VmentryMsrLoadCount = 0x00004014,
    VmentryInterruptionInfo = 0x00004016,
    VmentryExceptionErrCode = 0x00004018,
    VmentryInstructionLength = 0x0000401A,
    TprThreshold = 0x0000401C,
    SecondaryVmexecControls = 0x0000401E,
    PauseLoopExitingGap = 0x00004020,
    PauseLoopExitingWindow = 0x00004022,
    InstructionError = 0x00004400,
    VmexitReason = 0x00004402,
    VmexitInterruptionInfo = 0x00004404,
    VmexitInterruptionErrCode = 0x00004406,
    IdtVectoringInfo = 0x00004408,
    IdtVectoringErrCode = 0x0000440A,
    VmexitInstructionLength = 0x0000440C,
    VmexitInstructionInfo = 0x0000440E,
    GuestEsLimit = 0x00004800,
    GuestCsLimit = 0x00004802,
    GuestSsLimit = 0x00004804,
    GuestDsLimit = 0x00004806,
    GuestFsLimit = 0x00004808,
    GuestGsLimit = 0x0000480A,
    GuestLdtrLimit = 0x0000480C,
    GuestTrLimit = 0x0000480E,
    GuestGdtrLimit = 0x00004810,
    GuestIdtrLimit = 0x00004812,
    GuestEsAccessRights = 0x00004814,
    GuestCsAccessRights = 0x00004816,
    GuestSsAccessRights = 0x00004818,
    GuestDsAccessRights = 0x0000481A,
    GuestFsAccessRights = 0x0000481C,
    GuestGsAccessRights = 0x0000481E,
    GuestLdtrAccessRights = 0x00004820,
    GuestTrAccessRights = 0x00004822,
    GuestInterruptibilityState = 0x00004824,
    GuestActivityState = 0x00004826,
    GuestSmbase = 0x00004828,
    GuestIa32SysenterCsMsr = 0x0000482A,
    GuestPreemptionTimerKvalue = 0x0000482E,
    HostIa32SysenterCsMsr = 0x00004C00,
    Cr0GuestHostMask = 0x00006000,
    Cr4GuestHostMask = 0x00006002,
    Cr0ReadShadow = 0x00006004,
    Cr4ReadShadow = 0x00006006,
    Cr3Target0 = 0x00006008,
    Cr3Target1 = 0x0000600A,
    Cr3Target2 = 0x0000600C,
    Cr3Target3 = 0x0000600E,
    VmexitQualification = 0x00006400,
    IoRcx = 0x00006402,
    IoRsi = 0x00006404,
    IoRdi = 0x00006406,
    IoRip = 0x00006408,
    GuestLinearAddr = 0x0000640A,
    GuestCr0 = 0x00006800,
    GuestCr3 = 0x00006802,
    GuestCr4 = 0x00006804,
    GuestEsBase = 0x00006806,
    GuestCsBase = 0x00006808,
    GuestSsBase = 0x0000680A,
    GuestDsBase = 0x0000680C,
    GuestFsBase = 0x0000680E,
    GuestGsBase = 0x00006810,
    GuestLdtrBase = 0x00006812,
    GuestTrBase = 0x00006814,
    GuestGdtrBase = 0x00006816,
    GuestIdtrBase = 0x00006818,
    GuestDr7 = 0x0000681A,
    GuestRsp = 0x0000681C,
    GuestRip = 0x0000681E,
    GuestRflags = 0x00006820,
    GuestPendingDbgExceptions = 0x00006822,
    GuestIa32SysenterEspMsr = 0x00006824,
    GuestIa32SysenterEipMsr = 0x00006826,
    HostCr0 = 0x00006C00,
    HostCr3 = 0x00006C02,
    HostCr4 = 0x00006C04,
    HostFsBase = 0x00006C06,
    HostGsBase = 0x00006C08,
    HostTrBase = 0x00006C0A,
    HostGdtrBase = 0x00006C0C,
    HostIdtrBase = 0x00006C0E,
    HostIa32SysenterEspMsr = 0x00006C10,
    HostIa32SysenterEipMsr = 0x00006C12,
    HostRsp = 0x00006C14,
    HostRip = 0x00006C16,
}

impl Default for Vmcs {
    fn default() -> Self {
        Vmcs::new()
    }
}

impl Vmcs {
    /// Create a new Vmcs struct.
    pub fn new() -> Self {
        // Load vmx realated msrs.
        let vmx_basic = Msr::<IA32_VMX_BASIC>::read();
        let (rev_id, _vmcs_size) = (vmx_basic as u32, (vmx_basic >> 32) & 0xfff);

        Self {
            rev_id,
            indicator: 0,
            _data: [0; 0x1000 - 4],
        }
    }

    pub(crate) fn on(&self) -> Result<(), InstructionError> {
        unsafe {
            let err: i8;
            let pa = Kva::new(self as *const _ as usize)
                .unwrap()
                .into_pa()
                .into_usize();
            asm!(
                "clc",
                "vmxon [{}]",
                "setna {}",
                in(reg) &pa,
                out(reg_byte) err
            );
            if err != 0 {
                Err(Self::instruction_error())
            } else {
                Ok(())
            }
        }
    }

    /// Clear this VMCS.
    pub fn clear(&self) -> Result<(), VmError> {
        unsafe {
            let err: i8;
            let pa = Kva::new(self as *const _ as usize)
                .unwrap()
                .into_pa()
                .into_usize();
            asm!(
                "clc",
                "vmclear [{}]",
                "setna {}",
                in(reg) &pa,
                out(reg_byte) err
            );
            if err != 0 {
                Err(VmError::VmxOperationError(Vmcs::instruction_error()))
            } else {
                Ok(())
            }
        }
    }
    /// Make this VMCS as a working VMCS.
    pub fn activate(this: *mut Self) -> Result<ActiveVmcs, VmError> {
        unsafe {
            let err: i8;
            let pa = Kva::new(this as *const Self as usize)
                .unwrap()
                .into_pa()
                .into_usize();
            asm!(
                "clc",
                "vmptrld [{}]",
                "setna {}",
                in(reg) &pa,
                out(reg_byte) err
            );
            if err != 0 {
                Err(VmError::VmxOperationError(Self::instruction_error()))
            } else {
                Ok(ActiveVmcs { _p: () })
            }
        }
    }
    pub(crate) fn instruction_error() -> InstructionError {
        unsafe {
            let err: i8;
            let v: u64;
            asm!(
                "clc",
                "vmread {}, {}",
                "setna {}",
                out(reg) v,
                in(reg) Field::InstructionError as u64,
                out(reg_byte) err
            );
            if err != 0 {
                InstructionError::Unknown
            } else {
                match v {
                    1 => InstructionError::VmcallInVmxRoot,
                    2 => InstructionError::VmclearWithInvAddr,
                    3 => InstructionError::VmclearWithVmxon,
                    4 => InstructionError::VmresumeWithNonclearVmcs,
                    5 => InstructionError::VmresumeWithNonlaunchedVmcs,
                    6 => InstructionError::VmresumeAfterVmxoff,
                    7 => InstructionError::InvalidCs,
                    8 => InstructionError::InvalidHostState,
                    9 => InstructionError::VmPtrLdWithInvAddr,
                    10 => InstructionError::VmPtrLdWithVmxOn,
                    11 => InstructionError::VmPtrLdWithIncorrectRevId,
                    12 => InstructionError::UnsupportedVmcsField,
                    13 => InstructionError::WriteToRoField,
                    15 => InstructionError::VmxonInVmxRoot,
                    16 => InstructionError::VmEntryWithInvalidExecVmcs2,
                    17 => InstructionError::VmEntryWithNonlaucnhedExecVmcs2,
                    18 => InstructionError::VmEntryWithExecVmcs,
                    19 => InstructionError::VmcallWithNonclearVmcs,
                    20 => InstructionError::VmcallWithInvVmexitCs,
                    22 => InstructionError::VmcallWithIncorrectMsegRevId,
                    23 => InstructionError::VmxoffUnderDualMontiroTreatment,
                    24 => InstructionError::VmcallWithInvSmmMonitor,
                    25 => InstructionError::VmentryWithInvVmExecCs,
                    26 => InstructionError::VmentryWithEventBlockedByMovss,
                    28 => InstructionError::InvalidOperandToInveptInvvpid,
                    _ => InstructionError::Unknown,
                }
            }
        }
    }
}

/// A representation of active vmcs.
pub struct ActiveVmcs {
    _p: (),
}

impl ActiveVmcs {
    /// Get currently activated vmcs.
    pub fn activated() -> Result<(ActiveVmcs, Pa), VmError> {
        unsafe {
            let err: i8;
            let mut out: usize = 0;
            let ptr: *mut usize = &mut out as *mut _;
            asm!(
                "clc",
                "vmptrst [{}]",
                "setna {}",
                in(reg) ptr,
                out(reg_byte) err,
            );
            if err != 0 {
                Err(VmError::VmxOperationError(Vmcs::instruction_error()))
            } else {
                Ok((ActiveVmcs { _p: () }, Pa::new(out).unwrap()))
            }
        }
    }

    /// Dump the activated vmcs.
    pub fn dump(&self) {
        use abyss::x86_64::{Cr0, Cr4, segmentation::SegmentAccess};

        let (proc_ctl1, proc_ctl2) = unsafe {
            (
                VmcsProcBasedVmexecCtl::from_bits_truncate(
                    self.read(Field::ProcessorBasedVmexecControls).unwrap() as u32,
                ),
                VmcsProcBasedSecondaryVmexecCtl::from_bits_unchecked(
                    self.read(Field::SecondaryVmexecControls).unwrap() as u32,
                ),
            )
        };
        let cr0 = Cr0::from_bits_truncate(self.read(Field::GuestCr0).unwrap());
        let cr4 = Cr4::from_bits_truncate(self.read(Field::GuestCr4).unwrap());
        println!("Proc-based vm-exec control: {:?}", proc_ctl1);
        println!("Proc-based vm-exec control2: {:?}", proc_ctl2);
        println!(
            "RIP: {:x}, EFER: {:?}",
            self.read(Field::GuestRip).unwrap(),
            self.read(Field::GuestIa32Efer).unwrap()
        );
        println!(
            "cr0: {:?} cr3: {:x} cr4: {:?}",
            cr0,
            self.read(Field::GuestCr3).unwrap(),
            cr4
        );
        println!(
            "CS: base: {:x}, limit: {:x}, right: {:?}",
            self.read(Field::GuestCsBase).unwrap(),
            self.read(Field::GuestCsLimit).unwrap(),
            SegmentAccess::from_bits_truncate(self.read(Field::GuestCsAccessRights).unwrap() << 40)
        );
        println!(
            "ES: base: {:x}, limit: {:x}, right: {:?}",
            self.read(Field::GuestEsBase).unwrap(),
            self.read(Field::GuestEsLimit).unwrap(),
            SegmentAccess::from_bits_truncate(self.read(Field::GuestEsAccessRights).unwrap() << 40)
        );
        println!(
            "SS: base: {:x}, limit: {:x}, right: {:?}",
            self.read(Field::GuestSsBase).unwrap(),
            self.read(Field::GuestSsLimit).unwrap(),
            SegmentAccess::from_bits_truncate(self.read(Field::GuestSsAccessRights).unwrap() << 40)
        );
        println!(
            "DS: base: {:x}, limit: {:x}, right: {:?}",
            self.read(Field::GuestDsBase).unwrap(),
            self.read(Field::GuestDsLimit).unwrap(),
            SegmentAccess::from_bits_truncate(self.read(Field::GuestDsAccessRights).unwrap() << 40)
        );
        println!(
            "FS: base: {:x}, limit: {:x}, right: {:?}",
            self.read(Field::GuestFsBase).unwrap(),
            self.read(Field::GuestFsLimit).unwrap(),
            SegmentAccess::from_bits_truncate(self.read(Field::GuestFsAccessRights).unwrap() << 40)
        );
        println!(
            "GS: base: {:x}, limit: {:x}, right: {:?}",
            self.read(Field::GuestGsBase).unwrap(),
            self.read(Field::GuestGsLimit).unwrap(),
            SegmentAccess::from_bits_truncate(self.read(Field::GuestGsAccessRights).unwrap() << 40)
        );
    }

    /// Write to the vmcs field of the activated vmcs.
    pub fn write(&self, field: Field, v: u64) -> Result<(), VmError> {
        unsafe {
            let err: i8;
            asm!(
                "clc",
                "vmwrite {}, {}",
                "setna {}",
                in(reg) field as u64,
                in(reg) v,
                out(reg_byte) err
            );
            if err != 0 {
                Err(VmError::VmxOperationError(Vmcs::instruction_error()))
            } else {
                Ok(())
            }
        }
    }

    /// Read from the vmcs field of the activated vmcs.
    pub fn read(&self, field: Field) -> Result<u64, VmError> {
        unsafe {
            let err: i8;
            let v: u64;
            asm!(
                "clc",
                "vmread {}, {}",
                "setna {}",
                out(reg) v,
                in(reg) field as u64,
                out(reg_byte) err
            );
            if err != 0 {
                Err(VmError::VmxOperationError(Vmcs::instruction_error()))
            } else {
                Ok(v)
            }
        }
    }

    fn parse_basic_reason(&self, reason: u64) -> Result<BasicExitReason, VmError> {
        Ok(match reason {
            0x0 => BasicExitReason::ExceptionOrNmi,
            0x1 => {
                // 24.9.2 Information for VM Exits Due to Vectored Events
                let info = self.read(Field::VmexitInterruptionInfo)? as u32;
                BasicExitReason::ExternalInt(if info & 0x8000_0000 != 0 {
                    Some(ExternalIntInfo {
                        host_int: info as u8,
                        interruption_type: match (info >> 8) & 7 {
                            0 => InterruptionType::ExternalInt,
                            2 => InterruptionType::Nmi,
                            3 => InterruptionType::HardwareException,
                            5 => InterruptionType::PrivSoftwareException,
                            6 => InterruptionType::SoftwareExeception,
                            _ => unreachable!(),
                        },
                        error_code_valid: info & (1 << 11) != 0,
                        nmi_unblocked_by_iret: info & (1 << 12) != 0,
                    })
                } else {
                    None
                })
            }
            0x2 => BasicExitReason::TripleFault,
            0x3 => BasicExitReason::InitSignal,
            0x4 => BasicExitReason::StartupIpi,
            0x5 => BasicExitReason::IoSmi,
            0x6 => BasicExitReason::OtherSmi,
            0x7 => BasicExitReason::InterruptWindow,
            0x9 => BasicExitReason::TaskSwitch,
            0xA => BasicExitReason::Cpuid,
            0xC => BasicExitReason::Hlt,
            0xD => BasicExitReason::Invd,
            0xE => BasicExitReason::Invlpg,
            0xF => BasicExitReason::Rdpmc,
            0x10 => BasicExitReason::Rdtsc,
            0x11 => BasicExitReason::Rsm,
            0x12 => BasicExitReason::Vmcall,
            0x13 => BasicExitReason::Vmclear,
            0x14 => BasicExitReason::Vmlaunch,
            0x15 => BasicExitReason::Vmptrld,
            0x16 => BasicExitReason::Vmptrst,
            0x17 => BasicExitReason::Vmread,
            0x18 => BasicExitReason::Vmresume,
            0x19 => BasicExitReason::Vmwrite,
            0x1A => BasicExitReason::Vmxoff,
            0x1B => BasicExitReason::Vmxon,
            0x1C => BasicExitReason::MovCr,
            0x1D => BasicExitReason::MovDr,
            0x1E => BasicExitReason::IoInstruction,
            0x1F => BasicExitReason::Rdmsr,
            0x20 => BasicExitReason::Wrmsr,
            0x21 => BasicExitReason::EntfailGuestState,
            0x22 => BasicExitReason::EntfailMsrLoading,
            0x24 => BasicExitReason::Mwait,
            0x25 => BasicExitReason::Mtf,
            0x27 => BasicExitReason::Monitor,
            0x28 => BasicExitReason::Pause,
            0x29 => BasicExitReason::EntfailMachineChk,
            0x2B => BasicExitReason::TprBelowThreshold,
            0x2C => BasicExitReason::ApicAccess,
            0x2E => BasicExitReason::AccessGdtrOrIdtr,
            0x2F => BasicExitReason::AccessLdtrOrTr,
            0x30 => BasicExitReason::EptViolation {
                qualification: EptViolationQualification::from_bits_truncate(
                    self.read(Field::VmexitQualification)?,
                ),
                fault_addr: Gpa::new(self.read(Field::GuestPhysicalAddr)? as usize),
            },
            0x31 => BasicExitReason::EptMisconfig,
            0x32 => BasicExitReason::Invept,
            0x33 => BasicExitReason::Rdtscp,
            0x34 => BasicExitReason::VmxPreemptTimer,
            0x35 => BasicExitReason::Invvpid,
            0x36 => BasicExitReason::Wbinvd,
            0x37 => BasicExitReason::Xsetbv,
            _ => BasicExitReason::Unknown,
        })
    }

    /// Resolve the exit reason of the activated vmcs.
    pub fn exit_reason(&self) -> Result<ExitReason, VmError> {
        let reason = self.read(Field::VmexitReason)?;
        match reason {
            reason if reason & 0x20000000 != 0 => Ok(ExitReason::ExitFromVmxRootOperation(
                self.parse_basic_reason(reason & 0xffff)?,
            )),
            reason if reason & 0x80000000 != 0 => Ok(ExitReason::EntryFailure(
                self.parse_basic_reason(reason & 0xffff)?,
            )),
            reason => Ok(ExitReason::BasicExitReason(
                self.parse_basic_reason(reason)?,
            )),
        }
    }

    /// Get the instruction that rip pointed.
    pub fn get_instruction<P: Probe>(&self, p: &P) -> Result<Instruction, VmError> {
        let rip = self
            .read(Field::GuestRip)
            .map(|v| Gva::new(v as usize).expect("Invalid va for guest os."))?;
        let len = self.read(Field::VmexitInstructionLength)? as usize;
        assert!(len <= 11);
        // Every intel instruction is at most 11 bytes.
        let mut bytes = [0; 11];
        // Pull to the buffer.
        bytes[..len].copy_from_slice(unsafe {
            core::slice::from_raw_parts(
                p.gva2hva(self, rip)
                    .ok_or_else(|| {
                        VmError::ControllerError(Box::new(format!(
                            "Invalid memory access at {rip:?}",
                        )))
                    })?
                    .into_usize() as *const u8,
                len,
            )
        });

        let mut decoder = Decoder::with_ip(64, &bytes, 0, DecoderOptions::NONE);
        let mut insn = Instruction::default();
        if decoder.can_decode() {
            decoder.decode_out(&mut insn);
            Ok(insn)
        } else {
            Err(VmError::FailedToDecodeInstruction)
        }
    }

    /// Forward to the next instruction.
    pub fn forward_rip(&self) -> Result<(), VmError> {
        self.write(
            Field::GuestRip,
            self.read(Field::GuestRip)? + self.read(Field::VmexitInstructionLength)?,
        )
    }
}

/// Interruption type.
#[derive(Debug, Clone, Copy)]
pub enum InterruptionType {
    /// External interrupt.
    ExternalInt,
    /// Non-maskable interrupt.
    Nmi,
    /// Hardware exception.
    HardwareException,
    /// Privileged software exception.
    PrivSoftwareException,
    /// Software exception.
    SoftwareExeception,
}

/// External Interrupt Information
#[derive(Debug, Clone, Copy)]
pub struct ExternalIntInfo {
    /// Vector of interrupt or exception: BIT 7~0
    pub host_int: u8,
    /// Interruption type: BIT 10~8
    pub interruption_type: InterruptionType,
    /// Error code is valid: BIT 11.
    pub error_code_valid: bool,
    /// NMI unblocking due to IRET: BIT 12.
    pub nmi_unblocked_by_iret: bool,
}

/// Possible list of basic vmexit reasons.
///
/// See Table C-1. Basic Exit Reasons for details.
#[derive(Debug, Clone, Copy)]
#[allow(missing_docs)]
pub enum BasicExitReason {
    ExceptionOrNmi,
    /// External Int
    ///
    /// See Table 24-18. Format of the VM-Exit Interruption-Information Field
    ExternalInt(Option<ExternalIntInfo>),
    TripleFault,
    InitSignal,
    StartupIpi,
    IoSmi,
    OtherSmi,
    InterruptWindow,
    TaskSwitch,
    Cpuid,
    Hlt,
    Invd,
    Invlpg,
    Rdpmc,
    Rdtsc,
    Rsm,
    Vmcall,
    Vmclear,
    Vmlaunch,
    Vmptrld,
    Vmptrst,
    Vmread,
    Vmresume,
    Vmwrite,
    Vmxoff,
    Vmxon,
    MovCr,
    MovDr,
    IoInstruction,
    Rdmsr,
    Wrmsr,
    EntfailGuestState,
    EntfailMsrLoading,
    Mwait,
    Mtf,
    Monitor,
    Pause,
    EntfailMachineChk,
    TprBelowThreshold,
    ApicAccess,
    AccessGdtrOrIdtr,
    AccessLdtrOrTr,
    EptViolation {
        qualification: EptViolationQualification,
        fault_addr: Option<Gpa>,
    },
    EptMisconfig,
    Invept,
    Rdtscp,
    VmxPreemptTimer,
    Invvpid,
    Wbinvd,
    Xsetbv,
    Unknown,
}

bitflags::bitflags! {
    /// Exit Qualification for EPT Violations
    ///
    /// See Intel Manual volume 3C. Table 28-7. Exit Qualification for EPT Violations
    pub struct EptViolationQualification: u64 {
        /// Set if the access causing the EPT violation was a data read.
        const BIT0 = 1 << 0;
        ///  Set if the access causing the EPT violation was a data write.
        const BIT1 = 1 << 1;
        /// Set if the access causing the EPT violation was an instruction fetch.
        const BIT2 = 1 << 2;
        /// The logical-AND of bit 0 in the EPT paging-structure entries used to translate the guest-physical address of the
        /// access causing the EPT violation (indicates whether the guest-physical address was readable).
        const BIT3 = 1 << 3;
        /// The logical-AND of bit 1 in the EPT paging-structure entries used to translate the guest-physical address of the
        /// access causing the EPT violation (indicates whether the guest-physical address was writeable).
        const BIT4 = 1 << 4;
        /// The logical-AND of bit 2 in the EPT paging-structure entries used to translate the guest-physical address of the
        /// access causing the EPT violation.
        /// If the “mode-based execute control for EPT” VM-execution control is 0, this indicates whether the guest-physical
        /// address was executable. If that control is 1, this indicates whether the guest-physical address was executable
        /// for supervisor-mode linear addresses.
        const BIT5 = 1 << 5;
        /// If the “mode-based execute control” VM-execution control is 0, the value of this bit is undefined. If that control is
        /// 1, this bit is the logical-AND of bit 10 in the EPT paging-structure entries used to translate the guest-physical
        /// address of the access causing the EPT violation. In this case, it indicates whether the guest-physical address was
        /// executable for user-mode linear addresses.
        const BIT6 = 1 << 6;
        /// Set if the guest linear-address field is valid.
        /// The guest linear-address field is valid for all EPT violations except those resulting from an attempt to load the
        /// guest PDPTEs as part of the execution of the MOV CR instruction and those due to trace-address pre-translation
        /// (TAPT; Section 26.5.4).
        const BIT7 = 1 << 7;
        /// If bit 7 is 1:
        /// • Set if the access causing the EPT violation is to a guest-physical address that is the translation of a linear
        /// address.
        /// • Clear if the access causing the EPT violation is to a paging-structure entry as part of a page walk or the
        /// update of an accessed or dirty bit.
        /// Reserved if bit 7 is 0 (cleared to 0)
        const BIT8 = 1 << 8;
        /// If bit 7 is 1, bit 8 is 1, and the processor supports advanced VM-exit information for EPT violations,3 this bit is 0
        /// if the linear address is a supervisor-mode linear address and 1 if it is a user-mode linear address. (If CR0.PG = 0,
        /// the translation of every linear address is a user-mode linear address and thus this bit will be 1.) Otherwise, this
        /// bit is undefined.
        const BIT9 = 1 << 9;
        /// If bit 7 is 1, bit 8 is 1, and the processor supports advanced VM-exit information for EPT violations,3 this bit is 0
        /// if paging translates the linear address to a read-only page and 1 if it translates to a read/write page. (If CR0.PG =
        /// 0, every linear address is read/write and thus this bit will be 1.) Otherwise, this bit is undefined.
        const BIT10 = 1 << 10;
        /// If bit 7 is 1, bit 8 is 1, and the processor supports advanced VM-exit information for EPT violations,3 this bit is 0
        /// if paging translates the linear address to an executable page and 1 if it translates to an execute-disable page. (If
        /// CR0.PG = 0, CR4.PAE = 0, or IA32_EFER.NXE = 0, every linear address is executable and thus this bit will be 0.)
        /// Otherwise, this bit is undefined.
        const BIT11 = 1 << 11;
        /// NMI unblocking due to IRET (see Section 28.2.3).
        const BIT12 = 1 << 12;
        /// Set if the access causing the EPT violation was a shadow-stack access.
        const BIT13 = 1 << 13;
        /// If supervisor shadow-stack control is enabled (by setting bit 7 of EPTP), this bit is the same as bit 60 in the EPT
        /// paging-structure entry that maps the page of the guest-physical address of the access causing the EPT violation.
        /// Otherwise (or if translation of the guest-physical address terminates before reaching an EPT paging-structure
        /// entry that maps a page), this bit is undefined.
        const BIT14 = 1 << 14;
        /// This bit is set if the EPT violation was caused as a result of guest-paging verification. See Section 29.3.3.2.
        const BIT15 = 1 << 15;
        /// This bit is set if the access was asynchronous to instruction execution not the result of event delivery. The bit is
        /// set if the access is related to trace output by Intel PT (see Section 26.5.4), accesses related to PEBS on
        /// processors with the “EPT-friendly” enhancement (see Section 20.9.5), or to user-interrupt delivery (see Section
        /// 7.4.2). Otherwise, this bit is cleared.
        const BIT16 = 1 << 16;
    }
}

/// Enumeration of vmexit reasons.
#[derive(Debug, Clone, Copy)]
pub enum ExitReason {
    /// Exit during the run.
    BasicExitReason(BasicExitReason),
    /// Failed on vmlaunch or vmresume.
    EntryFailure(BasicExitReason),
    /// Exit from vmx root operation.
    ExitFromVmxRootOperation(BasicExitReason),
}

impl ExitReason {
    /// get basic exit reason of the exit reason.
    pub fn get_basic_reason(&self) -> &BasicExitReason {
        match self {
            Self::BasicExitReason(e)
            | Self::EntryFailure(e)
            | Self::ExitFromVmxRootOperation(e) => e,
        }
    }
}
