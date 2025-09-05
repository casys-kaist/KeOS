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

use super::UnwindError;
use core::arch::asm;
use core::convert::TryFrom;
use core::fmt::Debug;

#[derive(num_enum::TryFromPrimitive, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(usize)]
pub enum Register {
    Rax = 0,
    Rdx = 1,
    Rcx = 2,
    Rbx = 3,
    Rsi = 4,
    Rdi = 5,
    Rbp = 6,
    Rsp = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    R13 = 13,
    R14 = 14,
    R15 = 15,
    Rip = 16,
    Xmm0 = 17,
    Xmm1 = 18,
    Xmm2 = 19,
    Xmm3 = 20,
    Xmm4 = 21,
    Xmm5 = 22,
    Xmm6 = 23,
    Xmm7 = 24,
    Xmm8 = 25,
    Xmm9 = 26,
    Xmm10 = 27,
    Xmm11 = 28,
    Xmm12 = 29,
    Xmm13 = 30,
    Xmm14 = 31,
    Xmm15 = 32,
    /// CFA
    Cfa = 33,
}

#[allow(unused)]
impl Register {
    pub const CFA: Self = Self::Cfa;
    pub const IP: Self = Self::Rip;
    pub const BP: Self = Self::Rbp;
    #[allow(non_upper_case_globals)]
    pub const Sp: Self = Self::Rsp;
    pub const EH: Self = Self::Rax;
    pub const NUM_REGS: usize = 34;
    pub const DATA_REG1: Self = Self::Rax;
    pub const DATA_REG2: Self = Self::Rdx;

    #[inline]
    pub fn from_unwind_regnum(regnum: usize) -> Result<Self, UnwindError> {
        if regnum <= Self::Rip as usize {
            Self::try_from(regnum).map_err(|_| UnwindError::UnknownRegister)
        } else {
            Err(UnwindError::UnknownRegister)
        }
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct StackFrame {
    pub rax: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rbx: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rbp: usize,
    pub rsp: usize,
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
    /// rip.
    pub rip: usize,
}

impl Debug for StackFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        struct Hex(usize);
        impl Debug for Hex {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{:x}", self.0)
            }
        }
        f.debug_struct("StackFrame")
            .field("rax", &Hex(self.rax))
            .field("rbx", &Hex(self.rbx))
            .field("rcx", &Hex(self.rcx))
            .field("rdx", &Hex(self.rdx))
            .field("rsi", &Hex(self.rsi))
            .field("rdi", &Hex(self.rdi))
            .field("rbp", &Hex(self.rbp))
            .field("rsp", &Hex(self.rsp))
            .field("r8", &Hex(self.r8))
            .field("r9", &Hex(self.r9))
            .field("r10", &Hex(self.r10))
            .field("r11", &Hex(self.r11))
            .field("r12", &Hex(self.r12))
            .field("r13", &Hex(self.r13))
            .field("r14", &Hex(self.r14))
            .field("r15", &Hex(self.r15))
            .field("rip", &Hex(self.rip))
            .finish()
    }
}

impl StackFrame {
    #[inline(always)]
    pub const fn pc(&self) -> usize {
        self.rip
    }

    #[inline(always)]
    pub fn set_pc(&mut self, pc: usize) {
        self.rip = pc;
    }

    #[inline(always)]
    pub const fn sp(&self) -> usize {
        self.rsp
    }

    #[inline(always)]
    pub fn current() -> Self {
        unsafe {
            #[allow(clippy::uninit_assumed_init)]
            #[allow(invalid_value)]
            let mut o: StackFrame = core::mem::MaybeUninit::uninit().assume_init();
            asm!("lea rax, [rip]", out("rax") o.rip, options(nostack, nomem));
            asm!("",
                 out("rax") o.rax,
                 out("rdx") o.rdx,
                 out("rcx") o.rcx,
                 out("rsi") o.rsi,
                 out("rdi") o.rdi,
                 out("r8") o.r8,
                 out("r9") o.r9,
                 out("r10") o.r10,
                 out("r11") o.r11,
                 out("r12") o.r12,
                 out("r13") o.r13,
                 out("r14") o.r14,
                 out("r15") o.r15,
                 options(nostack, nomem)
            );
            asm!("mov rax, rbx", out("rax") o.rbx, options(nostack, nomem));
            asm!("mov rax, rsp", out("rax") o.rsp, options(nostack, nomem));
            asm!("mov rax, rbp", out("rax") o.rbp, options(nostack, nomem));
            o
        }
    }

    #[inline]
    pub fn get(&self, v: Register) -> Result<&usize, UnwindError> {
        match v {
            Register::Rax => Ok(&self.rax),
            Register::Rdx => Ok(&self.rdx),
            Register::Rcx => Ok(&self.rcx),
            Register::Rbx => Ok(&self.rbx),
            Register::Rsi => Ok(&self.rsi),
            Register::Rdi => Ok(&self.rdi),
            Register::Rbp => Ok(&self.rbp),
            Register::Rsp => Ok(&self.rsp),
            Register::R8 => Ok(&self.r8),
            Register::R9 => Ok(&self.r9),
            Register::R10 => Ok(&self.r10),
            Register::R11 => Ok(&self.r11),
            Register::R12 => Ok(&self.r12),
            Register::R13 => Ok(&self.r13),
            Register::R14 => Ok(&self.r14),
            Register::R15 => Ok(&self.r15),
            Register::Rip => Ok(&self.rip),
            _ => Err(UnwindError::UnmanagedRegister),
        }
    }

    #[inline]
    pub fn get_mut(&mut self, v: Register) -> Result<&mut usize, UnwindError> {
        match v {
            Register::Rax => Ok(&mut self.rax),
            Register::Rdx => Ok(&mut self.rdx),
            Register::Rcx => Ok(&mut self.rcx),
            Register::Rbx => Ok(&mut self.rbx),
            Register::Rsi => Ok(&mut self.rsi),
            Register::Rdi => Ok(&mut self.rdi),
            Register::Rbp => Ok(&mut self.rbp),
            Register::Rsp => Ok(&mut self.rsp),
            Register::R8 => Ok(&mut self.r8),
            Register::R9 => Ok(&mut self.r9),
            Register::R10 => Ok(&mut self.r10),
            Register::R11 => Ok(&mut self.r11),
            Register::R12 => Ok(&mut self.r12),
            Register::R13 => Ok(&mut self.r13),
            Register::R14 => Ok(&mut self.r14),
            Register::R15 => Ok(&mut self.r15),
            Register::Rip => Ok(&mut self.rip),
            _ => Err(UnwindError::UnmanagedRegister),
        }
    }
}
