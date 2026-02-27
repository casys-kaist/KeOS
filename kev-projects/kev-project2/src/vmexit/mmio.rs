//! Memory-mapped IO vmexit controller.

use alloc::{
    boxed::Box,
    collections::btree_map::{BTreeMap, Entry},
};
use core::cmp::Ordering;
use iced_x86::{Instruction, OpKind, Register};
use kev::{
    Probe, VmError,
    vcpu::{GeneralPurposeRegisters, GenericVCpuState, VmexitResult},
    vm::Gpa,
    vmcs::{BasicExitReason, EptViolationQualification, ExitReason, Field},
};

pub trait MmioHandler
where
    Self: Send + Sync,
{
    fn region(&self) -> MmioRegion;
    fn handle(
        &mut self,
        p: &dyn Probe,
        info: MmioInfo,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError>;
}

/// Representation of interval.
///
/// This implements interval tree by overloading PartialOrd, Ord, PartialEq, and
/// Eq with ordered map (BTreeMap).
#[derive(Eq, Clone, Copy)]
pub struct MmioRegion {
    // (start, end]
    pub start: Gpa,
    pub end: Gpa,
}

impl core::fmt::Debug for MmioRegion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(f, "MmioRegion({:?}, {:?}]", self.start, self.end)
    }
}

impl core::cmp::Ord for MmioRegion {
    fn cmp(&self, other: &Self) -> Ordering {
        let has_overlapping = self.start < other.end && self.end > other.start;
        if has_overlapping {
            Ordering::Equal
        } else {
            self.start.cmp(&other.start)
        }
    }
}

impl core::cmp::PartialOrd for MmioRegion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl core::cmp::PartialEq for MmioRegion {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl MmioRegion {
    /// Create a new MmioRegion.
    #[inline(always)]
    pub fn new(gpa: Gpa, size: usize) -> Self {
        Self {
            start: gpa,
            end: gpa + size,
        }
    }
}

/// Mmio vmexit controller.
#[derive(Default)]
pub struct Controller {
    inner: BTreeMap<MmioRegion, Box<dyn MmioHandler>>,
}

#[derive(Debug)]
pub enum Direction {
    Write8 { dst: Gpa, src: u8 },
    Write16 { dst: Gpa, src: u16 },
    Write32 { dst: Gpa, src: u32 },
    Write64 { dst: Gpa, src: u64 },
}

impl Direction {
    fn get_mmio_addr(&self) -> Gpa {
        match self {
            Self::Write8 { dst: v, .. }
            | Self::Write16 { dst: v, .. }
            | Self::Write32 { dst: v, .. }
            | Self::Write64 { dst: v, .. } => *v,
        }
    }
}

#[derive(Debug)]
pub struct MmioInfo {
    pub size: usize,
    pub direction: Direction,
}

impl Controller {
    /// Create a new mmio vmexit controller.
    pub fn new() -> Self {
        Controller {
            inner: BTreeMap::new(),
        }
    }
    /// Add a mmio region to the controller.
    pub fn register(&mut self, p: impl MmioHandler + 'static) {
        match self.inner.entry(p.region()) {
            Entry::Occupied(_) => panic!("overwrapping mmio region"),
            Entry::Vacant(v) => {
                v.insert(Box::new(p));
            }
        }
    }
}

impl kev::vmexits::VmexitController for Controller {
    fn handle<P: Probe>(
        &mut self,
        reason: ExitReason,
        p: &mut P,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        match reason.get_basic_reason() {
            BasicExitReason::EptViolation {
                qualification,
                fault_addr,
            } => {
                assert!(
                    qualification.contains(EptViolationQualification::BIT1),
                    "rip: {:x}, {qualification:?}, {fault_addr:?}",
                    generic_vcpu_state.vmcs.read(Field::GuestRip).unwrap()
                );

                let mmio_info = get_mmio_info(
                    generic_vcpu_state.gprs,
                    &generic_vcpu_state.vmcs.get_instruction(p)?,
                    fault_addr.ok_or(VmError::HandleVmexitFailed(reason))?,
                    *qualification,
                );
                if let Some(handler) = self.inner.get_mut(&MmioRegion::new(
                    mmio_info.direction.get_mmio_addr(),
                    mmio_info.size,
                )) {
                    handler
                        .handle(p, mmio_info, generic_vcpu_state)
                        .and_then(|r| generic_vcpu_state.vmcs.forward_rip().map(|_| r))
                } else {
                    Err(VmError::HandleVmexitFailed(reason))
                }
            }
            _ => Err(VmError::HandleVmexitFailed(reason)),
        }
    }
}

fn get_mmio_value(insn: &Instruction, gprs: &GeneralPurposeRegisters) -> u64 {
    match insn.op1_kind() {
        OpKind::Register => match insn.op1_register() {
            Register::AL => gprs.rax as u8 as u64,
            Register::BL => gprs.rbx as u8 as u64,
            Register::CL => gprs.rcx as u8 as u64,
            Register::DL => gprs.rdx as u8 as u64,
            Register::AX => gprs.rax as u16 as u64,
            Register::BX => gprs.rbx as u16 as u64,
            Register::CX => gprs.rcx as u16 as u64,
            Register::DX => gprs.rdx as u16 as u64,
            Register::EAX => gprs.rax as u32 as u64,
            Register::EBX => gprs.rbx as u32 as u64,
            Register::ECX => gprs.rcx as u32 as u64,
            Register::EDX => gprs.rdx as u32 as u64,
            Register::RAX => gprs.rax as u64,
            Register::RBX => gprs.rbx as u64,
            Register::RCX => gprs.rcx as u64,
            Register::RDX => gprs.rdx as u64,
            Register::R8L => gprs.r8 as u8 as u64,
            Register::R9L => gprs.r9 as u8 as u64,
            Register::R10L => gprs.r10 as u8 as u64,
            Register::R11L => gprs.r11 as u8 as u64,
            Register::R12L => gprs.r12 as u8 as u64,
            Register::R13L => gprs.r13 as u8 as u64,
            Register::R14L => gprs.r14 as u8 as u64,
            Register::R15L => gprs.r15 as u8 as u64,
            Register::R8W => gprs.r8 as u16 as u64,
            Register::R9W => gprs.r9 as u16 as u64,
            Register::R10W => gprs.r10 as u16 as u64,
            Register::R11W => gprs.r11 as u16 as u64,
            Register::R12W => gprs.r12 as u16 as u64,
            Register::R13W => gprs.r13 as u16 as u64,
            Register::R14W => gprs.r14 as u16 as u64,
            Register::R15W => gprs.r15 as u16 as u64,
            Register::R8D => gprs.r8 as u32 as u64,
            Register::R9D => gprs.r9 as u32 as u64,
            Register::R10D => gprs.r10 as u32 as u64,
            Register::R11D => gprs.r11 as u32 as u64,
            Register::R12D => gprs.r12 as u32 as u64,
            Register::R13D => gprs.r13 as u32 as u64,
            Register::R14D => gprs.r14 as u32 as u64,
            Register::R15D => gprs.r15 as u32 as u64,
            Register::R8 => gprs.r8 as u64,
            Register::R9 => gprs.r9 as u64,
            Register::R10 => gprs.r10 as u64,
            Register::R11 => gprs.r11 as u64,
            Register::R12 => gprs.r12 as u64,
            Register::R13 => gprs.r13 as u64,
            Register::R14 => gprs.r14 as u64,
            Register::R15 => gprs.r15 as u64,
            e => todo!("{e:?}"),
        },
        OpKind::Immediate8 => insn.immediate8() as u64,
        OpKind::Immediate16 => insn.immediate16() as u64,
        OpKind::Immediate32 => insn.immediate32() as u64,
        OpKind::Immediate64 => insn.immediate64(),
        OpKind::Immediate8to16 => insn.immediate8() as u64,
        OpKind::Immediate8to32 => insn.immediate16() as u64,
        OpKind::Immediate8to64 => insn.immediate16() as u64,
        OpKind::Immediate32to64 => insn.immediate32() as u64,
        e => unreachable!("{e:?}"),
    }
}

fn get_mmio_info(
    gprs: &mut GeneralPurposeRegisters,
    insn: &Instruction,
    fault_addr: Gpa,
    _qualification: EptViolationQualification,
) -> MmioInfo {
    // Must be write access.
    assert!(insn.op0_kind() == OpKind::Memory);

    let size = insn.memory_size();

    let direction = match size {
        iced_x86::MemorySize::UInt8 => Direction::Write8 {
            dst: fault_addr,
            src: get_mmio_value(insn, gprs) as u8,
        },
        iced_x86::MemorySize::UInt16 => Direction::Write16 {
            dst: fault_addr,
            src: get_mmio_value(insn, gprs) as u16,
        },
        iced_x86::MemorySize::UInt32 => Direction::Write32 {
            dst: fault_addr,
            src: get_mmio_value(insn, gprs) as u32,
        },
        iced_x86::MemorySize::UInt64 => Direction::Write64 {
            dst: fault_addr,
            src: get_mmio_value(insn, gprs),
        },
        _ => unreachable!(),
    };

    let size: usize = match size {
        iced_x86::MemorySize::UInt8 => 8,
        iced_x86::MemorySize::UInt16 => 16,
        iced_x86::MemorySize::UInt32 => 32,
        iced_x86::MemorySize::UInt64 => 64,
        _ => unreachable!(),
    };
    MmioInfo { size, direction }
}
