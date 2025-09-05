// Copyright 2025 Computer Architecture and Systems Lab
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::ehframe::CommonInformationEntry;
use super::reader::{Peeker, get_sleb128, get_uleb128};
use super::{Register, UnwindBacktrace, UnwindError};
use arrayvec::ArrayVec;
use core::convert::TryFrom;

#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug, num_enum::TryFromPrimitive)]
pub enum OperandType {
    Addr = 0x03,
    Deref = 0x06,
    Const1u = 0x08,
    Const1s = 0x09,
    Const2u = 0x0a,
    Const2s = 0x0b,
    Const4u = 0x0c,
    Const4s = 0x0d,
    Const8u = 0x0e,
    Const8s = 0x0f,
    Constu = 0x10,
    Consts = 0x11,
    Dup = 0x12,
    Drop = 0x13,
    Over = 0x14,
    Pick = 0x15,
    Swap = 0x16,
    Rot = 0x17,
    Xderef = 0x18,
    Abs = 0x19,
    And = 0x1a,
    Div = 0x1b,
    Minus = 0x1c,
    Mod = 0x1d,
    Mul = 0x1e,
    Neg = 0x1f,
    Not = 0x20,
    Or = 0x21,
    Plus = 0x22,
    PlusUconst = 0x23,
    Shl = 0x24,
    Shr = 0x25,
    Shra = 0x26,
    Xor = 0x27,
    Skip = 0x2f,
    Bra = 0x28,
    Eq = 0x29,
    Ge = 0x2a,
    Gt = 0x2b,
    Le = 0x2c,
    Lt = 0x2d,
    Ne = 0x2e,
    Lit0 = 0x30,
    Lit1 = 0x31,
    Lit2 = 0x32,
    Lit3 = 0x33,
    Lit4 = 0x34,
    Lit5 = 0x35,
    Lit6 = 0x36,
    Lit7 = 0x37,
    Lit8 = 0x38,
    Lit9 = 0x39,
    Lit10 = 0x3a,
    Lit11 = 0x3b,
    Lit12 = 0x3c,
    Lit13 = 0x3d,
    Lit14 = 0x3e,
    Lit15 = 0x3f,
    Lit16 = 0x40,
    Lit17 = 0x41,
    Lit18 = 0x42,
    Lit19 = 0x43,
    Lit20 = 0x44,
    Lit21 = 0x45,
    Lit22 = 0x46,
    Lit23 = 0x47,
    Lit24 = 0x48,
    Lit25 = 0x49,
    Lit26 = 0x4a,
    Lit27 = 0x4b,
    Lit28 = 0x4c,
    Lit29 = 0x4d,
    Lit30 = 0x4e,
    Lit31 = 0x4f,
    Reg0 = 0x50,
    Reg1 = 0x51,
    Reg2 = 0x52,
    Reg3 = 0x53,
    Reg4 = 0x54,
    Reg5 = 0x55,
    Reg6 = 0x56,
    Reg7 = 0x57,
    Reg8 = 0x58,
    Reg9 = 0x59,
    Reg10 = 0x5a,
    Reg11 = 0x5b,
    Reg12 = 0x5c,
    Reg13 = 0x5d,
    Reg14 = 0x5e,
    Reg15 = 0x5f,
    Reg16 = 0x60,
    Reg17 = 0x61,
    Reg18 = 0x62,
    Reg19 = 0x63,
    Reg20 = 0x64,
    Reg21 = 0x65,
    Reg22 = 0x66,
    Reg23 = 0x67,
    Reg24 = 0x68,
    Reg25 = 0x69,
    Reg26 = 0x6a,
    Reg27 = 0x6b,
    Reg28 = 0x6c,
    Reg29 = 0x6d,
    Reg30 = 0x6e,
    Reg31 = 0x6f,
    Breg0 = 0x70,
    Breg1 = 0x71,
    Breg2 = 0x72,
    Breg3 = 0x73,
    Breg4 = 0x74,
    Breg5 = 0x75,
    Breg6 = 0x76,
    Breg7 = 0x77,
    Breg8 = 0x78,
    Breg9 = 0x79,
    Breg10 = 0x7a,
    Breg11 = 0x7b,
    Breg12 = 0x7c,
    Breg13 = 0x7d,
    Breg14 = 0x7e,
    Breg15 = 0x7f,
    Breg16 = 0x80,
    Breg17 = 0x81,
    Breg18 = 0x82,
    Breg19 = 0x83,
    Breg20 = 0x84,
    Breg21 = 0x85,
    Breg22 = 0x86,
    Breg23 = 0x87,
    Breg24 = 0x88,
    Breg25 = 0x89,
    Breg26 = 0x8a,
    Breg27 = 0x8b,
    Breg28 = 0x8c,
    Breg29 = 0x8d,
    Breg30 = 0x8e,
    Breg31 = 0x8f,
    Regx = 0x90,
    Fbreg = 0x91,
    Bregx = 0x92,
    Piece = 0x93,
    DerefSize = 0x94,
    XDerefSize = 0x95,
    Nop = 0x96,
    PushObjectAddress = 0x97,
    Call2 = 0x98,
    Call4 = 0x99,
    CallRef = 0x9a,
    LoUser = 0xe0,
    HiUser = 0xff,
}

#[repr(u8)]
#[derive(Debug, num_enum::TryFromPrimitive)]
pub enum Opcode {
    AdvanceLoc = 0x40,
    Offset = 0x80,
    Restore = 0xc0,
    Nop = 0x00,
    SetLoc = 0x01,
    AdvanceLoc1 = 0x02,
    AdvanceLoc2 = 0x03,
    AdvanceLoc4 = 0x04,
    OffsetExtended = 0x05,
    RestoreExtended = 0x06,
    Undefined = 0x07,
    SameValue = 0x08,
    Register = 0x09,
    RememberState = 0x0a,
    RestoreState = 0x0b,
    DefCfa = 0x0c,
    DefCfaRegister = 0x0d,
    DefCfaOffset = 0x0e,
    DefCfaExpression = 0x0f,
    Expression = 0x10,
    OffsetExtendedSf = 0x11,
    DefCfaSf = 0x12,
    DefCfaOffsetSf = 0x13,
    ValExpression = 0x16,
    LoUser = 0x1c,
    MipsAdvanceLoc8 = 0x1d,
    GnuWindowSave = 0x2d,
    GnuArgsSize = 0x2e,
    GnuNegativeOffsetExtended = 0x2f,
    HiUser = 0x3c,
}

#[derive(Debug)]
pub struct Operations<'a> {
    ops: &'a [u8],
}

impl<'a> Operations<'a> {
    pub fn new(ops: &'a [u8]) -> Self {
        Self { ops }
    }

    pub fn run_from_result(
        &self,
        end_ip: usize,
        result: ExecutionResult,
        cie: &CommonInformationEntry,
    ) -> Result<ExecutionResult, UnwindError> {
        let ip = result.ip;
        let mut cursor = ExecutionCursor {
            cie,
            ops: self,
            pos: 0,
            initial: result,
            result,
            _args_size: None,
            ip,
            stack: ArrayVec::new(),
        };

        while cursor.ip <= end_ip && cursor.step()? {}
        Ok(cursor.result)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Delta {
    Undef,
    Same,
    CfaRelative(usize),
    Reg(usize),
    /* Expr(usize),
     * ValueExpr(usize), */
}

#[derive(Copy, Clone, Debug)]
pub struct RegisterStates {
    cfa_off_column: usize,
    return_address_column: Register,
    deltas: [Delta; Register::NUM_REGS + 1],
}

#[derive(Copy, Clone, Debug)]
pub struct ExecutionResult {
    state: RegisterStates,
    ip: usize,
}

impl ExecutionResult {
    pub fn new(return_address_column: Register, ip: usize) -> Self {
        ExecutionResult {
            state: RegisterStates {
                cfa_off_column: 0,
                return_address_column,
                deltas: [Delta::Same; Register::NUM_REGS + 1],
            },
            ip,
        }
    }

    pub fn apply<T>(&self, ctx: &mut UnwindBacktrace<T>) -> Result<(), UnwindError>
    where
        T: Peeker,
    {
        ctx.cfa = match self.state.deltas[ExecutionCursor::DWARF_CFA_REG_COLUMN] {
            Delta::Reg(s)
                if s == Register::Sp as usize
                    && matches!(self.state.deltas.get(s), Some(&Delta::Same)) =>
            {
                let cfa = ctx.cfa.overflowing_add(self.state.cfa_off_column).0;
                *ctx.frame.get_mut(Register::Sp)? = cfa;
                cfa
            }
            Delta::Reg(s) => {
                Register::from_unwind_regnum(s)
                    .and_then(|r| Ok(*ctx.frame.get(r)?))?
                    .overflowing_add(self.state.cfa_off_column)
                    .0
            }
            Delta::Undef | Delta::CfaRelative(_) | Delta::Same => {
                return Err(UnwindError::InvalidApplication);
            }
        };

        for (reg, delta) in self.state.deltas.iter().enumerate() {
            if let Ok(register) = Register::from_unwind_regnum(reg) {
                match delta {
                    Delta::Undef => *ctx.frame.get_mut(register)? = 0,
                    Delta::Same => (),
                    Delta::Reg(r) => {
                        *ctx.frame.get_mut(register)? = Register::from_unwind_regnum(*r)
                            .and_then(|r| Ok(*ctx.frame.get(r)?))?;
                    }
                    Delta::CfaRelative(v) => {
                        *ctx.frame.get_mut(register)? =
                            ctx.read_mem(ctx.cfa.overflowing_add(*v).0)?;
                    }
                }
            }
        }

        ctx.frame
            .set_pc(*ctx.frame.get(self.state.return_address_column)?);
        Ok(())
    }
}

pub struct ExecutionCursor<'a> {
    cie: &'a CommonInformationEntry,
    ops: &'a Operations<'a>,
    pos: usize,
    ip: usize,
    initial: ExecutionResult,
    result: ExecutionResult,
    _args_size: Option<usize>,
    stack: ArrayVec<RegisterStates, 0x100>,
}

impl ExecutionCursor<'_> {
    const DWARF_CFA_REG_COLUMN: usize = Register::NUM_REGS;
    const OPERAND_MASK: u8 = 0x3f;
    const OPCODE_MASK: u8 = 0xc0;

    #[inline]
    fn pick_u8(&mut self) -> Result<u8, UnwindError> {
        let out = self.ops.ops.get(self.pos).cloned();
        if let Some(out) = out {
            self.pos += 1;
            Ok(out)
        } else {
            Err(UnwindError::ParsingFailure)
        }
    }

    #[inline]
    fn pick_u16(&mut self) -> Result<u16, UnwindError> {
        if let (Some(out1), Some(out2)) = (
            self.ops.ops.get(self.pos).cloned(),
            self.ops.ops.get(self.pos + 1).cloned(),
        ) {
            self.pos += 2;
            Ok(u16::from_ne_bytes([out1, out2]))
        } else {
            Err(UnwindError::ParsingFailure)
        }
    }

    #[inline]
    fn pick_u32(&mut self) -> Result<u32, UnwindError> {
        if let (Some(out1), Some(out2), Some(out3), Some(out4)) = (
            self.ops.ops.get(self.pos).cloned(),
            self.ops.ops.get(self.pos + 1).cloned(),
            self.ops.ops.get(self.pos + 2).cloned(),
            self.ops.ops.get(self.pos + 3).cloned(),
        ) {
            self.pos += 4;
            Ok(u32::from_ne_bytes([out1, out2, out3, out4]))
        } else {
            Err(UnwindError::ParsingFailure)
        }
    }

    #[inline]
    fn pick_uleb128(&mut self) -> Result<usize, UnwindError> {
        get_uleb128(|| self.pick_u8().ok()).ok_or(UnwindError::ParsingFailure)
    }

    #[inline]
    fn pick_sleb128(&mut self) -> Result<isize, UnwindError> {
        get_sleb128(|| self.pick_u8().ok()).ok_or(UnwindError::ParsingFailure)
    }

    #[inline]
    fn register(&mut self, v: usize) -> Result<usize, UnwindError> {
        if v < Register::NUM_REGS {
            Ok(v)
        } else {
            Err(UnwindError::BadRegister)
        }
    }

    #[allow(clippy::question_mark)]
    fn step(&mut self) -> Result<bool, UnwindError> {
        let op = if let Ok(op) = self.pick_u8() {
            op
        } else {
            return Ok(false);
        };
        let (op, operand) = if op & Self::OPCODE_MASK != 0 {
            (
                Opcode::try_from(op & !Self::OPERAND_MASK)
                    .map_err(|_| UnwindError::BadOpcode(op & !Self::OPERAND_MASK))?,
                Some(op & Self::OPERAND_MASK),
            )
        } else {
            (
                Opcode::try_from(op).map_err(|_| UnwindError::BadOpcode(op))?,
                None,
            )
        };
        match op {
            Opcode::AdvanceLoc => {
                self.ip += (operand.unwrap() as usize) * self.cie.code_align_factor;
            }
            Opcode::Offset => {
                let regnum = self.register(operand.unwrap() as usize)?;
                self.result.state.deltas[regnum] = Delta::CfaRelative(
                    (self.pick_uleb128()? as isize * self.cie.data_align_factor) as usize,
                );
            }
            Opcode::Restore => {
                let regnum = self.register(operand.unwrap() as usize)?;
                self.result.state.deltas[regnum] = self.initial.state.deltas[regnum];
            }
            Opcode::Nop => (),
            // Opcode::SetLoc => { self.result.ip = self.read_with_encoding() }
            Opcode::AdvanceLoc1 => {
                self.ip = self
                    .ip
                    .overflowing_add(
                        (self.pick_u8()? as usize)
                            .overflowing_mul(self.cie.code_align_factor)
                            .0,
                    )
                    .0;
            }
            Opcode::AdvanceLoc2 => {
                self.ip = self
                    .ip
                    .overflowing_add(
                        (self.pick_u16()? as usize)
                            .overflowing_mul(self.cie.code_align_factor)
                            .0,
                    )
                    .0;
            }
            Opcode::AdvanceLoc4 => {
                self.ip = self
                    .ip
                    .overflowing_add(
                        (self.pick_u32()? as usize)
                            .overflowing_mul(self.cie.code_align_factor)
                            .0,
                    )
                    .0;
            }
            Opcode::OffsetExtended => {
                let regnum = self.pick_uleb128().and_then(|e| self.register(e))?;
                self.result.state.deltas[regnum] = Delta::CfaRelative(
                    (self.pick_uleb128()? as isize)
                        .overflowing_mul(self.cie.data_align_factor)
                        .0 as usize,
                );
            }
            Opcode::RestoreExtended => {
                let regnum = self.pick_uleb128().and_then(|e| self.register(e))?;
                self.result.state.deltas[regnum] = self.initial.state.deltas[regnum];
            }
            Opcode::Undefined => {
                let regnum = self.pick_uleb128().and_then(|e| self.register(e))?;
                self.result.state.deltas[regnum] = Delta::Undef;
            }
            Opcode::SameValue => {
                let regnum = self.pick_uleb128().and_then(|e| self.register(e))?;
                self.result.state.deltas[regnum] = Delta::Same
            }
            Opcode::Register => {
                let regnum = self.pick_uleb128().and_then(|e| self.register(e))?;
                let from_reg = self.pick_uleb128().and_then(|e| self.register(e));
                assert!(from_reg.is_ok());
                self.result.state.deltas[regnum] = Delta::Reg(from_reg?);
            }
            Opcode::RememberState => self.stack.push(self.result.state),
            Opcode::RestoreState => match self.stack.pop() {
                Some(state) => {
                    self.result.state = state;
                }
                _ => {
                    return Err(UnwindError::InvalidOp(Opcode::RestoreState as u8));
                }
            },
            Opcode::DefCfa => {
                let regnum = self.pick_uleb128().and_then(|e| self.register(e))?;
                self.result.state.deltas[Self::DWARF_CFA_REG_COLUMN] = Delta::Reg(regnum);
                self.result.state.cfa_off_column = self.pick_uleb128()?;
            }
            Opcode::DefCfaRegister => {
                let regnum = self.pick_uleb128().and_then(|e| self.register(e))?;
                self.result.state.deltas[Self::DWARF_CFA_REG_COLUMN] = Delta::Reg(regnum);
            }
            Opcode::DefCfaOffset => {
                self.result.state.cfa_off_column = self.pick_uleb128()?;
            }
            // Opcode::DefCfaExpression,
            // Opcode::Expression,
            Opcode::OffsetExtendedSf => {
                let regnum = self.pick_uleb128().and_then(|e| self.register(e))?;
                self.result.state.deltas[regnum] = Delta::CfaRelative(
                    self.pick_sleb128()?
                        .overflowing_mul(self.cie.data_align_factor)
                        .0 as usize,
                );
            }
            Opcode::DefCfaSf => {
                let regnum = self.pick_uleb128().and_then(|e| self.register(e))?;
                self.result.state.deltas[Self::DWARF_CFA_REG_COLUMN] = Delta::Reg(regnum);
                self.result.state.cfa_off_column = self
                    .pick_sleb128()?
                    .overflowing_mul(self.cie.data_align_factor)
                    .0 as usize;
            }
            Opcode::DefCfaOffsetSf => {
                self.result.state.cfa_off_column = self
                    .pick_sleb128()?
                    .overflowing_mul(self.cie.data_align_factor)
                    .0 as usize;
            }
            // Opcode::ValExpression,
            Opcode::LoUser => return Err(UnwindError::BadOpcode(Opcode::LoUser as u8)),
            #[cfg(target_arch = "mips")]
            Opcode::MipsAdvanceLoc8 => {
                unimplemented!("UNIMPLEMENTED: {:?}", op);
            }
            #[cfg(not(target_arch = "mips"))]
            Opcode::MipsAdvanceLoc8 => {
                return Err(UnwindError::BadOpcode(Opcode::MipsAdvanceLoc8 as u8));
            }
            #[cfg(target_arch = "sparc")]
            Opcode::GnuWindowSave => {
                unimplemented!("UNIMPLEMENTED: {:?}", op);
            }
            #[cfg(not(target_arch = "sparc"))]
            Opcode::GnuWindowSave => {
                return Err(UnwindError::BadOpcode(Opcode::GnuWindowSave as u8));
            }
            Opcode::GnuArgsSize => {
                self._args_size = Some(self.pick_uleb128()?);
            }
            Opcode::GnuNegativeOffsetExtended => {
                let regnum = self.pick_uleb128().and_then(|e| self.register(e))?;
                self.result.state.deltas[regnum] = Delta::CfaRelative(
                    -(self.pick_uleb128()? as isize * self.cie.data_align_factor) as usize,
                );
            }
            Opcode::HiUser => return Err(UnwindError::BadOpcode(Opcode::LoUser as u8)),
            _ => {
                unimplemented!("UNIMPLEMENTED: {:?}", op);
            }
        }
        Ok(true)
    }
}
