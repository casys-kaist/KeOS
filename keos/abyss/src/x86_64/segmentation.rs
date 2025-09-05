//! Segmentation.

use super::PrivilegeLevel;
use core::arch::asm;

use super::table::{GlobalDescriptorTable, LocalDescriptorTable, SystemTableRegister};
use super::tss::TaskStateSegment;

bitflags::bitflags! {
    /// X86_64's access permission of segment.
    pub struct SegmentAccess: u64 {
        /// Granularity
        const G = 1 << 55;
        /// Default operation size (0 = 16-bit segment; 1 = 32-bit segment)
        const D_B = 1 << 54;
        /// 64-bit code segment (IA-32e mode only)
        const L = 1 << 53;
        /// Available for use by system software
        const AVL = 1 << 52;

        /// Segment present
        const P = 1 << 47;
        /// Descriptor type  (0 = system; 1 = code or data).
        const S = 1 << 44;
        /// Data or Code
        const CODE = 1 << 43;
        /// Expand_down/Conforming.
        const EC = 1 << 42;
        /// Writable/Readable.
        const WR = 1 << 41;
        /// Accessed.
        const A = 1 << 40;
    }
}

impl SegmentAccess {
    const BASE_31_24_SHIFT: u64 = 56;
    const SEG_LIMIT_SHIFT: u64 = 48;
    const DPL_SHIFT: u64 = 45;
    const BASE_23_0_SHIFT: u64 = 16;
}

/// X86_64's Segment Descriptor.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct SegmentDescriptor(u64);

impl SegmentDescriptor {
    /// Create a null segment.
    #[inline]
    pub const fn null() -> Self {
        Self::new(0, 0, SegmentAccess::empty(), PrivilegeLevel::Ring0)
    }

    /// Create a new segment.
    #[inline]
    pub const fn new(base: u64, limit: u64, access: SegmentAccess, dpl: PrivilegeLevel) -> Self {
        let (limit_15_0, base_23_0, access, dpl, limit_23_16, base_31_24) = (
            limit & 0xffff,
            base & 0xff_ffff,
            access.bits(),
            dpl as u64,
            (limit >> 16) & 0xf,
            (base >> 24) & 0xff,
        );

        Self(
            limit_15_0
                | base_23_0 << SegmentAccess::BASE_23_0_SHIFT
                | access
                | dpl << SegmentAccess::DPL_SHIFT
                | limit_23_16 << SegmentAccess::SEG_LIMIT_SHIFT
                | base_31_24 << SegmentAccess::BASE_31_24_SHIFT,
        )
    }
}

bitflags::bitflags! {
    /// X86_64's access permission of 64bit segment.
    pub struct SegmentAccess64: u64 {
        /// Granularity
        const G = 1 << 55;
        /// Available for use by system software
        const AVL = 1 << 52;

        /// Segment present
        const P = 1 << 47;
        /// Data or Code
        const CODE = 1 << 43;
        /// Expand_down/Conforming.
        const EC = 1 << 42;
        /// Writable/Readable.
        const WR = 1 << 41;
        /// Accessed.
        const A = 1 << 40;
        /// Available 64-bit TSS
        const T64A = 0x9 << Self::TYPE_SHIFT;
        /// Busy 64-bit TSS
        const T64B = 0xB << Self::TYPE_SHIFT;
        /// 64-bit Call Gate
        const CG64 = 0xC << Self::TYPE_SHIFT;
        /// 64-bit Interrupt Gate
        const IG64 = 0xE << Self::TYPE_SHIFT;
        /// 64-bit Trap Gate
        const TG64 = 0xF << Self::TYPE_SHIFT;

    }
}

impl SegmentAccess64 {
    const BASE_31_24_SHIFT: u64 = 56;
    const SEG_LIMIT_SHIFT: u64 = 48;
    const DPL_SHIFT: u64 = 45;
    const BASE_23_0_SHIFT: u64 = 16;
    const TYPE_SHIFT: u64 = 40;
}

/// X86_64's 64bit Segment Descriptor.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SegmentDescriptor64(u64, u64);

impl SegmentDescriptor64 {
    /// Create a null segment.
    #[inline]
    pub const fn null() -> Self {
        Self::new(0, 0, SegmentAccess64::empty(), PrivilegeLevel::Ring0)
    }

    /// Create a new segment.
    #[inline]
    pub const fn new(base: u64, limit: u64, access: SegmentAccess64, dpl: PrivilegeLevel) -> Self {
        let (limit_15_0, base_23_0, access, dpl, limit_23_16, base_31_24, base_63_32, clear) = (
            limit & 0xffff,
            base & 0xff_ffff,
            access.bits(),
            dpl as u64,
            (limit >> 16) & 0xf,
            (base >> 24) & 0xff,
            (base >> 32) & 0xffff_ffff,
            0,
        );

        Self(
            limit_15_0
                | base_23_0 << SegmentAccess64::BASE_23_0_SHIFT
                | access
                | dpl << SegmentAccess64::DPL_SHIFT
                | limit_23_16 << SegmentAccess64::SEG_LIMIT_SHIFT
                | base_31_24 << SegmentAccess64::BASE_31_24_SHIFT,
            base_63_32 | clear << 40,
        )
    }
}

/// X86_64's segment selector.
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct SegmentSelector(u16);

impl core::fmt::Debug for SegmentSelector {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        f.debug_struct("SegmentSelector")
            .field("index", &self.index())
            .field("dpl", &self.dpl())
            .finish()
    }
}

impl SegmentSelector {
    /// Create a new SegmentSelector from the index and dpl.
    #[inline]
    pub const fn new(index: u16, dpl: PrivilegeLevel) -> Self {
        Self((index << 3) | dpl as u16)
    }

    /// Pack the SegmentSelector into a word.
    #[inline]
    pub const fn pack(self) -> u16 {
        self.0
    }

    /// Get index of the SegmentSelector.
    #[inline]
    pub const fn index(self) -> u16 {
        self.0 >> 3
    }

    /// Get dpl of the SegmentSelector.
    #[inline]
    pub const fn dpl(self) -> PrivilegeLevel {
        match self.0 & 3 {
            0 => PrivilegeLevel::Ring0,
            1 => PrivilegeLevel::Ring1,
            2 => PrivilegeLevel::Ring2,
            3 => PrivilegeLevel::Ring3,
            _ => unreachable!(),
        }
    }
}

/// X86_64's Segment Register
pub enum SegmentRegister {
    /// Code Segment.
    Cs,
    /// Data Segment.
    Ds,
    /// Stack Segment.
    Ss,
    /// Extra Segment.
    Es,
    /// Extra Segment (E -> F).
    Fs,
    /// Extra Segment (F -> G).
    Gs,
    /// Task State Segment.
    Tss,
}

impl SegmentRegister {
    /// Load the segment selector into the segment register.
    #[inline(always)]
    pub fn load(&self, ss: SegmentSelector) {
        unsafe {
            match self {
                Self::Cs => {
                    // XXX: unique_id_per asm is not supported yet.
                    // we need to use att syntax as rex.W prefix is not supported.
                    #[inline(never)]
                    unsafe fn do_cs_reload(ss: SegmentSelector) {
                        unsafe {
                            asm!("push {0}",
                             "movabs $1f, %rax",
                             "push %rax",
                             "lretq",
                             "1:",
                             in(reg) ss.pack() as u64,
                             out("rax") _,
                             options(att_syntax))
                        }
                    }

                    do_cs_reload(ss)
                }
                Self::Ds => asm!(
                    "mov ds, {:x}",
                    in(reg) ss.pack(),
                    options(nostack, nomem)),
                Self::Ss => asm!(
                    "mov ss, {:x}",
                    in(reg) ss.pack(),
                    options(nostack, nomem)),
                Self::Es => asm!(
                    "mov es, {:x}",
                    in(reg) ss.pack(),
                    options(nostack, nomem)),
                Self::Fs => asm!(
                    "mov fs, {:x}",
                    in(reg) ss.pack(),
                    options(nostack, nomem)),
                Self::Gs => asm!(
                    "mov gs, {:x}",
                    in(reg) ss.pack(),
                    options(nostack, nomem)),
                Self::Tss => asm!(
                    "ltr {:x}",
                    in(reg) ss.pack(),
                    options(nostack, nomem)),
            }
        }
    }
}

/// A table for segment descriptors.
#[repr(C)]
pub struct SegmentTable {
    _null: SegmentDescriptor,
    kernel_code: SegmentDescriptor,
    kernel_data: SegmentDescriptor,
    user_data: SegmentDescriptor,
    user_code: SegmentDescriptor,
    tss: [SegmentDescriptor64; crate::MAX_CPU],
}

/// Types of segment.
#[derive(Copy, Clone, Debug)]
pub enum Segment {
    /// Null segment.
    Null,
    /// Kernel code segment (KC).
    KernelCode,
    /// Kernel data segment (KD).
    KernelData,
    /// User data segment (UD).
    UserData,
    /// User code segment (UC).
    UserCode,
    /// Task-state-struct segment (TSS).
    Tss,
}

const TSS_INIT: (super::tss::TaskStateSegment, usize) = (super::tss::TaskStateSegment::empty(), 0);

#[doc(hidden)]
#[unsafe(no_mangle)]
pub static mut TSS: [(super::tss::TaskStateSegment, usize); crate::MAX_CPU] =
    [TSS_INIT; crate::MAX_CPU];

impl Segment {
    /// Segment selector for Kernel Code.
    pub const KERNEL_CODE_SELECTOR: SegmentSelector =
        SegmentSelector::new(1, PrivilegeLevel::Ring0);
    /// Segment selector for Kernel Data.
    pub const KERNEL_DATA_SELECTOR: SegmentSelector =
        SegmentSelector::new(2, PrivilegeLevel::Ring0);
    /// Segment selector for User Data.
    pub const USER_DATA_SELECTOR: SegmentSelector = SegmentSelector::new(3, PrivilegeLevel::Ring3);
    /// Segment selector for User Code.
    pub const USER_CODE_SELECTOR: SegmentSelector = SegmentSelector::new(4, PrivilegeLevel::Ring3);

    /// Get segment selector from the [`Segment`].
    #[inline]
    pub fn into_selector(self) -> SegmentSelector {
        let cpuid = crate::x86_64::intrinsics::cpuid();
        match self {
            Self::Null => SegmentSelector::new(0, PrivilegeLevel::Ring3),
            Self::KernelCode => Self::KERNEL_CODE_SELECTOR,
            Self::KernelData => Self::KERNEL_DATA_SELECTOR,
            Self::UserData => Self::USER_DATA_SELECTOR,
            Self::UserCode => Self::USER_CODE_SELECTOR,
            Self::Tss => SegmentSelector::new(5 + 2 * cpuid as u16, PrivilegeLevel::Ring0),
        }
    }
}

impl SegmentTable {
    /// Initialize tss segment.
    ///
    /// # SAFETY
    /// Interrupt must be blocked.
    pub(crate) unsafe fn init_tss(&'static mut self) {
        unsafe {
            let cpuid = crate::x86_64::intrinsics::cpuid();
            let tss = &mut TSS[cpuid].0;

            (tss as *mut TaskStateSegment)
                .as_mut()
                .unwrap()
                .fill_segment_descriptor(&mut self.tss[cpuid]);

            SegmentRegister::Tss.load(Segment::Tss.into_selector());
        }
    }

    /// Update the tss.
    ///
    /// # SAFETY
    /// Interrupt must be blocked.
    #[inline]
    pub unsafe fn update_tss(v: usize) {
        unsafe {
            let cpuid = crate::x86_64::intrinsics::cpuid();
            TSS[cpuid].0.rsp0 = v;
        }
    }

    /// Get current tss address.
    pub unsafe fn current_tss() -> &'static mut TaskStateSegment {
        unsafe {
            let cpuid = crate::x86_64::intrinsics::cpuid();
            &mut TSS[cpuid].0
        }
    }

    /// Load this table into global descriptor table and segment registers.
    #[inline]
    pub fn load(&'static self) {
        SystemTableRegister::new(self).load::<GlobalDescriptorTable>();
        SegmentRegister::Gs.load(Segment::UserData.into_selector());
        SegmentRegister::Fs.load(Segment::UserData.into_selector());
        SegmentRegister::Es.load(Segment::Null.into_selector());
        SegmentRegister::Ds.load(Segment::Null.into_selector());
        SegmentRegister::Ss.load(Segment::KernelData.into_selector());
        SegmentRegister::Cs.load(Segment::KernelCode.into_selector());
        LocalDescriptorTable::kill();
    }
}

/// A unique kernel segment table.
pub static mut SEGMENT_TABLE: SegmentTable = SegmentTable {
    _null: SegmentDescriptor::null(),
    kernel_code: SegmentDescriptor::new(
        0,
        0xffffffff,
        SegmentAccess::from_bits_truncate(
            SegmentAccess::P.bits()
            | SegmentAccess::S.bits()
            | SegmentAccess::L.bits()
            | SegmentAccess::G.bits()
            // Code, readable.
            | SegmentAccess::CODE.bits()
            | SegmentAccess::WR.bits(),
        ),
        PrivilegeLevel::Ring0,
    ),
    kernel_data: SegmentDescriptor::new(
        0,
        0xffffffff,
        SegmentAccess::from_bits_truncate(
            SegmentAccess::P.bits()
            | SegmentAccess::S.bits()
            | SegmentAccess::L.bits()
            | SegmentAccess::G.bits()
            // Data, writable.
            | SegmentAccess::WR.bits(),
        ),
        PrivilegeLevel::Ring0,
    ),
    user_data: SegmentDescriptor::new(
        0,
        0xffffffff,
        SegmentAccess::from_bits_truncate(
            SegmentAccess::P.bits()
            | SegmentAccess::S.bits()
            | SegmentAccess::L.bits()
            | SegmentAccess::G.bits()
            // Data, writable.
            | SegmentAccess::WR.bits(),
        ),
        PrivilegeLevel::Ring3,
    ),
    user_code: SegmentDescriptor::new(
        0,
        0xffffffff,
        SegmentAccess::from_bits_truncate(
            SegmentAccess::P.bits()
            | SegmentAccess::S.bits()
            | SegmentAccess::L.bits()
            | SegmentAccess::G.bits()
            // Code, readable.
            | SegmentAccess::CODE.bits()
            | SegmentAccess::WR.bits(),
        ),
        PrivilegeLevel::Ring3,
    ),
    tss: [SegmentDescriptor64::null(); crate::MAX_CPU],
};
