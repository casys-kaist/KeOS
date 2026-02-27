//! Port-mapped IO vmexit controller.
//!
//! Port-mapped I/O is a method of communication between external devices and
//! the CPU, where each device is assigned a unique port address that is used as
//! an operand in the in/out instruction family. This allows the CPU to directly
//! read or write data to the devices. There are different variations of the
//! in/out instructions, depending on the type and size of the operands used.
//!
//! In virtual machine management, the VMM can choose to allow or trap
//! port-mapped I/O instructions executed by the guest. However, in this
//! section, all I/O instructions are trapped to the host, giving the VMM
//! control over their behavior to hand-on the concept of the instruction
//! emulation.
//!
//! Instruction emulation contains two steps; 1) decode the "semantic" of the
//! instruction; 2) emulate the "semantic" of the instruction. Through the
//! implementation, you could follow the way that traditional virtualization
//! emulates the instruction.
//!
//! ## Tasks
//! In this part, You need to emulate the 18 in/out instructions.
//! The given template conducts the step 2, emulation.
//!
//! Decode the instruction and initialize the [`IoInstruction`] instance with
//! the corresponding opcode and registers. The controller will then use this
//! information to forward the request to the appropriate handler. When handling
//! Outsw_DX_m8, Outsw_DX_m16 and Outsd_DX_m32, it is necessary to copy the
//! memory contents by translating guest virtual address to host virtual
//! address. You can translate the guest address to the host address by
//! [`Probe`].
use alloc::{
    boxed::Box,
    collections::btree_map::{BTreeMap, Entry},
    format,
};
use iced_x86::{Code, Instruction};
use kev::{
    Probe, VmError,
    vcpu::{GenericVCpuState, Rflags, VmexitResult},
    vm::Gva,
    vmcs::{BasicExitReason, ExitReason, Field},
};

/// Trait that represent handlers for port-mapped devices.
pub trait PioHandler
where
    Self: Send + Sync,
{
    /// handle I/O instructions on the device indicated by the port with the
    /// operands included in direction.
    fn handle(
        &self,
        port: u16,
        direction: Direction,
        p: &dyn Probe,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError>;
}

/// Pio vmexit controller.
pub struct Controller {
    pios: BTreeMap<u16, Box<dyn PioHandler>>,
}

impl Controller {
    /// Create a new pio controller.
    pub fn new() -> Self {
        Self {
            pios: BTreeMap::new(),
        }
    }
    /// Insert pio handler to the index.
    ///
    /// Return false if pio handler for index is exists.
    /// Otherwise, return true.
    pub fn register(&mut self, port: u16, pio: impl PioHandler + 'static) -> bool {
        match self.pios.entry(port) {
            Entry::Occupied(_) => false,
            Entry::Vacant(v) => {
                v.insert(Box::new(pio));
                true
            }
        }
    }
}

impl kev::vmexits::VmexitController for Controller {
    fn handle<P: kev::Probe>(
        &mut self,
        reason: ExitReason,
        p: &mut P,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, kev::VmError> {
        match reason.get_basic_reason() {
            BasicExitReason::IoInstruction => {
                let insn = generic_vcpu_state.vmcs.get_instruction(p)?;
                self.handle_ioinsn(insn, p, generic_vcpu_state)
                    .and_then(|s| generic_vcpu_state.vmcs.forward_rip().map(|_| s))
            }
            _ => Err(kev::VmError::HandleVmexitFailed(reason)),
        }
    }
}

#[derive(Debug)]
/// Possible prefix of io instruction.
pub enum Prefix {
    /// Has none of prefix.
    None,
    /// Has repe or rep prefix.
    Rep,
    /// Has repne.
    Repne,
}

#[derive(Debug)]
/// Direction and the Value of the instruction
pub enum Direction {
    /// Input byte from I/O port into AL.
    InbAl,
    /// Input word from I/O port into AX.
    InwAx,
    /// Input double word from I/O port into EAX.
    IndEax,
    /// Input byte from I/O port into memory.
    Inbm(Gva),
    /// Input word from I/O port into memory.
    Inwm(Gva),
    /// Input double word from I/O port into memory.
    Indm(Gva),
    /// Output a byte (1 byte)
    Outb(u8),
    /// Output a word (2 bytes)
    Outw(u16),
    /// Output a double word (4 bytes)
    Outd(u32),
}

#[derive(Debug)]
/// The decoded information of io instruction.
pub struct IoInstruction {
    /// Port of the io iostruction
    pub port: u16,
    /// Direction and value of the io instruction
    pub direction: Direction,
}

impl Controller {
    fn handle_ioinsn_one<P: Probe>(
        &self,
        insn: Instruction,
        p: &mut P,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        // Hint
        //   - Use [`Probe::gva2hva`] to translate guest memory to host memory.
        let IoInstruction { port, direction } = match insn.code() {
            // -- in families.
            // in al, dx
            // Input byte from I/O port in DX into AL.
            Code::In_AL_DX => todo!(),
            // in ax, dx
            // Input word from I/O port in DX into AX.
            Code::In_AX_DX => todo!(),
            // in eax, dx
            // Input doubleword from I/O port in DX into EAX.
            Code::In_EAX_DX => todo!(),
            // in al, imm8
            // Input byte from imm8 I/O port address into AL.
            Code::In_AL_imm8 => todo!(),
            // in ax, imm8
            // Input word from imm8 I/O port address into AX.
            Code::In_AX_imm8 => todo!(),
            // in eax, imm8
            // Input dword from imm8 I/O port address into EAX.
            Code::In_EAX_imm8 => todo!(),
            // insb
            // Input byte from I/O port specified in DX into memory location specified in ES:(E)DI
            // or RDI.*
            Code::Insb_m8_DX => todo!(),
            // insw
            // Input word from I/O port specified in DX into memory location specified in ES:(E)DI
            // or RDI.1
            Code::Insw_m16_DX => todo!(),
            // insd
            // Input doubleword from I/O port specified in DX into memory location specified in
            // ES:(E)DI or RDI.1
            Code::Insd_m32_DX => todo!(),
            // -- out families.
            // out dx, al
            // Output byte from AL into I/O port in DX.
            Code::Out_DX_AL => todo!(),
            // out dx, ax
            // Output word from AX into I/O port in DX.
            Code::Out_DX_AX => todo!(),
            // out dx, eax
            // Output double word from AX into I/O port in DX.
            Code::Out_DX_EAX => todo!(),
            // out imm8, al
            // Output byte from AL into I/O port in imm8.
            Code::Out_imm8_AL => todo!(),
            // out imm8, ax
            // Output byte from AL into I/O port in imm8.
            Code::Out_imm8_AX => todo!(),
            // out imm8, eax
            // Output double word from EAX into I/O port in imm8.
            Code::Out_imm8_EAX => todo!(),
            // outsb
            // Output byte from memory location specified in DS:(E)SI or RSI to I/O port specified
            // in DX**.
            Code::Outsb_DX_m8 => todo!(),
            // outsw
            // Output word from memory location specified in DS:(E)SI or RSI to I/O port specified
            // in DX**.
            Code::Outsw_DX_m16 => todo!(),
            // outsd
            // Output doubleword from memory location specified in DS:(E)SI or RSI to I/O port
            // specified in DX**.
            Code::Outsd_DX_m32 => todo!(),
            _ => unreachable!(),
        };
        let result = if let Some(handler) = self.pios.get(&port) {
            handler.handle(port, direction, p, generic_vcpu_state)
        } else {
            Err(VmError::ControllerError(Box::new(format!(
                "Unknown io port: 0x{port:x}"
            ))))
        };
        let df = Rflags::from_bits_truncate(generic_vcpu_state.vmcs.read(Field::GuestRflags)?)
            .contains(Rflags::DF);
        match insn.code() {
            Code::Insb_m8_DX if !df => {
                generic_vcpu_state.gprs.rdi = generic_vcpu_state.gprs.rdi.overflowing_add(1).0
            }
            Code::Insb_m8_DX if df => {
                generic_vcpu_state.gprs.rdi = generic_vcpu_state.gprs.rdi.overflowing_sub(1).0
            }
            Code::Insw_m16_DX if !df => {
                generic_vcpu_state.gprs.rdi = generic_vcpu_state.gprs.rdi.overflowing_add(2).0
            }
            Code::Insw_m16_DX if df => {
                generic_vcpu_state.gprs.rdi = generic_vcpu_state.gprs.rdi.overflowing_sub(2).0
            }
            Code::Insd_m32_DX if !df => {
                generic_vcpu_state.gprs.rdi = generic_vcpu_state.gprs.rdi.overflowing_add(4).0
            }
            Code::Insd_m32_DX if df => {
                generic_vcpu_state.gprs.rdi = generic_vcpu_state.gprs.rdi.overflowing_sub(4).0
            }
            Code::Outsb_DX_m8 if !df => {
                generic_vcpu_state.gprs.rsi = generic_vcpu_state.gprs.rsi.overflowing_add(1).0
            }
            Code::Outsb_DX_m8 if df => {
                generic_vcpu_state.gprs.rsi = generic_vcpu_state.gprs.rsi.overflowing_sub(1).0
            }
            Code::Outsw_DX_m16 if !df => {
                generic_vcpu_state.gprs.rsi = generic_vcpu_state.gprs.rsi.overflowing_add(2).0
            }
            Code::Outsw_DX_m16 if df => {
                generic_vcpu_state.gprs.rsi = generic_vcpu_state.gprs.rsi.overflowing_sub(2).0
            }
            Code::Outsd_DX_m32 if !df => {
                generic_vcpu_state.gprs.rsi = generic_vcpu_state.gprs.rsi.overflowing_add(4).0
            }
            Code::Outsd_DX_m32 if df => {
                generic_vcpu_state.gprs.rsi = generic_vcpu_state.gprs.rsi.overflowing_sub(4).0
            }
            _ => (),
        }
        result
    }

    fn handle_ioinsn<P: Probe>(
        &self,
        insn: Instruction,
        p: &mut P,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        if insn.has_rep_prefix() || insn.has_repne_prefix() {
            while generic_vcpu_state.gprs.rcx != 0 {
                let result = self.handle_ioinsn_one(insn, p, generic_vcpu_state);
                generic_vcpu_state.gprs.rcx -= 1;
                match result {
                    Ok(VmexitResult::Ok) => (),
                    r => return r,
                }
            }
            Ok(VmexitResult::Ok)
        } else {
            self.handle_ioinsn_one(insn, p, generic_vcpu_state)
        }
    }
}
