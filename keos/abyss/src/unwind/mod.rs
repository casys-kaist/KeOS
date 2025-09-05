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

//! A pure rust implementation of Unwind project.
//!
//! <https://refencodings.linuxfoundation.org/LSB_1.3.0/gLSB/gLSB/ehframehdr.html>

#![allow(internal_features)]

mod ehframe;
mod machine;
mod personality;
mod reader;
mod x86_64;

use ehframe::EhFrameHeader;
use x86_64::Register;

pub use ehframe::FrameDescriptionEntry;
pub use reader::{DwarfReader, Encoding, Peeker};
pub use x86_64::StackFrame;

pub enum ExceptionHandlingPhase {
    Search,
    Cleanup,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum PersonalityResult {
    Continue,
    Run(usize),
    Error,
    Stop,
}

pub enum UnwindError {
    BadRegister,
    BadOpcode(u8),
    BadOperand(u8),
    InvalidOp(u8),
    InvalidApplication,
    InvalidPc(usize),
    UnknownRegister,
    UnmanagedRegister,
    ParsingFailure,
    UnwindablePc(usize),
    MemoryOutOfBound(usize, core::ops::Range<usize>),
    PersonalityFailure,
}

impl core::fmt::Debug for UnwindError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadRegister => write!(f, "BadRegister"),
            Self::BadOpcode(v) => write!(f, "BadOpcode(0x{v:x})"),
            Self::BadOperand(v) => write!(f, "BadOperand(0x{v:x})"),
            Self::InvalidOp(v) => write!(f, "InvalidOp(0x{v:x})"),
            Self::InvalidApplication => write!(f, "InvalidApplication"),
            Self::InvalidPc(v) => write!(f, "InvalidPc(0x{v:x})"),
            Self::UnknownRegister => write!(f, "UnknownRegister"),
            Self::UnmanagedRegister => write!(f, "UnmanagedRegister"),
            Self::ParsingFailure => write!(f, "ParsingFailure"),
            Self::UnwindablePc(v) => write!(f, "UnwindablePc(0x{v:x})"),
            Self::MemoryOutOfBound(v, _) => write!(f, "MemoryOutOfBound(0x{v:x})"),
            Self::PersonalityFailure => write!(f, "PersonalityFailure"),
        }
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct UnwindBacktrace<T>
where
    T: Peeker,
{
    pub frame: StackFrame,
    sp_range: core::ops::Range<usize>,
    cfa: usize,
    reader: DwarfReader<T>,
}

impl<T> UnwindBacktrace<T>
where
    T: Peeker,
{
    #[inline(never)]
    pub fn new(
        frame: StackFrame,
        sp_range: core::ops::Range<usize>,
        reader: DwarfReader<T>,
    ) -> Self {
        let cfa = frame.sp();
        UnwindBacktrace {
            frame,
            sp_range,
            cfa,
            reader,
        }
    }

    #[inline]
    pub fn read_mem(&self, addr: usize) -> Result<usize, UnwindError> {
        if self.sp_range.contains(&addr) {
            Ok(unsafe { (addr as *const usize).as_ref() }.cloned().unwrap())
        } else {
            Err(UnwindError::MemoryOutOfBound(addr, self.sp_range.clone()))
        }
    }

    #[inline]
    fn do_unwind_frame<UnwindFn>(mut self, mut unwind_fn: UnwindFn) -> Result<(), UnwindError>
    where
        UnwindFn: FnMut(Self, &FrameDescriptionEntry) -> (Self, bool),
    {
        let hdr = EhFrameHeader::parse(self.reader.clone());
        let (mut previous_pc, mut previous_cfa) = (self.frame.pc(), self.cfa);
        while self.frame.pc() != 0 {
            let fde = hdr
                .find(self.frame.pc())
                .unwrap()
                .insn
                .parse(self.reader.clone())
                .ok_or(UnwindError::ParsingFailure)?;
            if fde.pc.contains(&self.frame.pc()) || (self.frame.pc() >> 16 == 0x40)
            /* "Known" PC for userprog */
            {
                let (s, is_stop) = unwind_fn(self, &fde);
                if is_stop {
                    return Ok(());
                }
                self = s;
                fde.run(self.frame.pc())?.apply(&mut self)?;
                if self.frame.pc() == previous_pc && self.cfa == previous_cfa {
                    return Err(UnwindError::UnwindablePc(self.frame.pc()));
                }

                self.frame.set_pc(self.frame.pc().wrapping_sub(1));
                previous_pc = self.frame.pc();
                previous_cfa = self.cfa;
            } else {
                return Err(UnwindError::InvalidPc(self.frame.pc()));
            }
        }
        Ok(())
    }

    #[inline]
    pub fn unwind_frame<UnwindFn>(self, mut unwind_fn: UnwindFn) -> Result<(), UnwindError>
    where
        UnwindFn: FnMut(&Self, &FrameDescriptionEntry),
    {
        self.do_unwind_frame(|this, fde| {
            unwind_fn(&this, fde);
            (this, false)
        })
    }

    /// Raise exception through unwind with hook function.
    ///
    /// # Safety
    /// LSDA in Unwind table must point the valid handler address.
    #[inline]
    pub unsafe fn run<S, BacktraceFn>(
        self,
        state: &mut S,
        mut backtrace: BacktraceFn,
    ) -> Result<(), UnwindError>
    where
        BacktraceFn: FnMut(&mut S, &Self, &FrameDescriptionEntry),
    {
        unsafe {
            let mut action_after_search = PersonalityResult::Continue;

            self.do_unwind_frame(|this, fde| {
                backtrace(state, &this, fde);
                // Call personality if available.
                if let Some(personality) = fde.cie.personality {
                    let routine = personality
                        as *const fn(
                            ExceptionHandlingPhase,
                            &FrameDescriptionEntry,
                            &StackFrame,
                        ) -> PersonalityResult;
                    match (*routine)(ExceptionHandlingPhase::Search, fde, &this.frame) {
                        PersonalityResult::Error => action_after_search = PersonalityResult::Error,
                        PersonalityResult::Stop => return (this, true),
                        _ => (),
                    }
                }
                (this, false)
            })
        }
    }
}
