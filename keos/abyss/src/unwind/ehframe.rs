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

use super::machine::{ExecutionResult, Operations};
use super::reader::{Application, DwarfReader, Encoding, Peeker};
use super::{Register, UnwindError};

pub struct EhFrameHeader<T>
where
    T: Peeker,
{
    _eh_frame_ptr: usize,
    fde_count: usize,
    bst_parser_snapshot: DwarfReader<T>,
    bst_entry_encoding: Encoding,
}

impl<T> EhFrameHeader<T>
where
    T: Peeker,
{
    pub fn parse(mut parser: DwarfReader<T>) -> Self {
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct RawHeader {
            /// Version. must be 1.
            version: u8,
            /// The encoding format of the eh_frame_ptr field.
            eh_frame_ptr_enc: u8,
            /// The encoding format of the fde_count field. A value of
            /// DW_EH_PE_omit indicates the binary search table is
            /// not present.
            fde_count_enc: u8,
            /// The encoding format of the entries in the binary search table. A
            /// value of DW_EH_PE_omit indicates the binary search
            /// table is not present.
            table_enc: u8,
        }

        let hdr: RawHeader = parser.read().unwrap();
        assert_eq!(hdr.version, 1);

        let (eh_frame_ptr_enc, fde_count_enc, table_enc) = (
            Encoding::from(hdr.eh_frame_ptr_enc),
            Encoding::from(hdr.fde_count_enc),
            Encoding::from(hdr.table_enc),
        );

        let eh_frame_ptr = parser
            .read_with_encoding(eh_frame_ptr_enc)
            .expect("Ehframe is not founded.");

        let fde_count = parser
            .read_with_encoding(fde_count_enc)
            .expect("Fde entry is not founded.");
        assert!(fde_count * table_enc.size() * 2 + parser.current() <= parser.end());

        Self {
            _eh_frame_ptr: eh_frame_ptr,
            fde_count,
            bst_parser_snapshot: parser,
            bst_entry_encoding: table_enc,
        }
    }

    pub fn get(&self, idx: usize) -> Option<UnwindIndex> {
        if self.fde_count > idx {
            let mut parser = self.bst_parser_snapshot.clone();
            parser.wheel((idx * self.bst_entry_encoding.size() * 2) as isize);

            Some(UnwindIndex {
                addr_offset: parser
                    .read_with_encoding(self.bst_entry_encoding)
                    .expect("Fail to read addr_offset."),
                insn: FrameDescriptionEntryPointer(
                    parser
                        .read_with_encoding(self.bst_entry_encoding)
                        .expect("Fail to read insn."),
                ),
            })
        } else {
            None
        }
    }

    pub fn find(&self, pc: usize) -> Option<UnwindIndex> {
        let (mut lo, mut hi) = (0, self.fde_count - 1);
        while lo < hi - 1 {
            let mid = lo + (hi - lo) / 2;
            if self.get(mid)?.addr_offset <= pc {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        self.get(lo)
    }
}

pub struct UnwindIndex {
    pub addr_offset: usize,
    pub insn: FrameDescriptionEntryPointer,
}

impl core::fmt::Debug for UnwindIndex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(
            f,
            "UnwindIndex {{ addr_offset: {:016x}, insn: {:016x} }}",
            self.addr_offset, self.insn.0
        )
    }
}

// http://refspecs.linux-foundation.org/LSB_4.1.0/LSB-Core-generic/LSB-Core-generic.html#EHFRAMEHDR

#[derive(Debug, Clone, Copy)]
pub struct CommonInformationEntryPointer(usize);

#[derive(Debug)]
pub struct CommonInformationEntry {
    pub cie_id: u32,
    pub code_align_factor: usize,
    pub data_align_factor: isize,
    pub ret_addr_column: u8,
    pub initial_instructions: Operations<'static>,
    pub lsda_encoding: Option<Encoding>,
    pub fde_encoding: Option<Encoding>,
    pub personality: Option<usize>,
}

impl CommonInformationEntryPointer {
    const CIE_VERSION: u8 = 1;

    pub fn parse<T>(&self, mut reader: DwarfReader<T>) -> Option<CommonInformationEntry>
    where
        T: Peeker,
    {
        reader.reset(self.0);
        let length = match reader.read::<u32>()? {
            0xffffffff => reader.read::<u64>()?,
            v => v as u64,
        };
        let new_end = reader.pos + length as usize;
        reader.set_end(new_end);

        let cie_id = reader.read::<u32>()?;
        if Self::CIE_VERSION != reader.read::<u8>()? {
            return None;
        }

        // Just use max 4 bytes.
        let mut augmentation = [0; 4];
        let mut index = 0;
        loop {
            let e = reader.read::<u8>()?;
            match e {
                0 => break,
                b'z' | b'L' | b'P' | b'R' if index < augmentation.len() => {
                    augmentation[index] = e;
                    index += 1;
                }
                _ => return None,
            }
        }

        let (code_align_factor, data_align_factor, ret_addr_column) = (
            reader.read_uleb128()?,
            reader.read_sleb128()?,
            reader.read::<u8>()?,
        );

        let (mut lsda_encoding, mut personality, mut fde_encoding) = (None, None, None);

        if index > 0 && augmentation[0] == b'z' {
            let augmentation_length = reader.read_uleb128()?;
            let max_offset = reader.pos + augmentation_length;
            for c in &augmentation[1..index] {
                match c {
                    // No more augmentation data exists, but expect more.
                    // LSDA
                    b'L' => lsda_encoding = reader.read::<u8>().map(Encoding::from),
                    // Personality routine.
                    b'P' => {
                        personality = reader
                            .read::<u8>()
                            .map(Encoding::from)
                            .and_then(|encoding| reader.read_with_encoding(encoding))
                    }
                    // FDE pointer.
                    b'R' => fde_encoding = reader.read::<u8>().map(Encoding::from),
                    _ => return None,
                }
                if reader.pos > max_offset {
                    return None;
                }
            }
        }

        let initial_instruction_length = new_end - reader.pos;
        Some(CommonInformationEntry {
            cie_id,
            code_align_factor,
            data_align_factor,
            ret_addr_column,
            initial_instructions: Operations::new(unsafe {
                core::slice::from_raw_parts(
                    reader.current() as *const u8,
                    initial_instruction_length,
                )
            }),
            lsda_encoding,
            fde_encoding,
            personality,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameDescriptionEntryPointer(usize);

#[derive(Debug)]
pub struct FrameDescriptionEntry {
    pub cie: CommonInformationEntry,
    pub pc: core::ops::Range<usize>,
    pub lsda: Option<usize>,
    pub instructions: Operations<'static>,
}

impl FrameDescriptionEntryPointer {
    pub fn parse<T>(&self, mut reader: DwarfReader<T>) -> Option<FrameDescriptionEntry>
    where
        T: Peeker,
    {
        reader.reset(self.0);
        let length = match reader.read::<u32>()? {
            0xffffffff => reader.read::<u64>()?,
            v => v as u64,
        };
        let new_end = reader.pos + length as usize;
        reader.set_end(new_end);

        let cie = CommonInformationEntryPointer(reader.current() - reader.read::<u32>()? as usize)
            .parse(reader.clone())?;
        let encoding = cie.fde_encoding?;
        let (pc_begin, pc_len) = (
            reader.read_with_encoding(encoding)?,
            reader.read_with_encoding(Encoding {
                format: encoding.format,
                application: Application::Absoulte,
            })?,
        );
        let augmentation_length = reader.read_uleb128()?;
        let cfi_loc = augmentation_length + reader.pos;
        let lsda = cie
            .lsda_encoding
            .and_then(|encoding| reader.read_with_encoding(encoding));
        if reader.pos == cfi_loc {
            let instruction_length = new_end - reader.pos;
            Some(FrameDescriptionEntry {
                cie,
                pc: pc_begin..pc_begin + pc_len,
                lsda,
                instructions: Operations::new(unsafe {
                    core::slice::from_raw_parts(reader.current() as *const u8, instruction_length)
                }),
            })
        } else {
            None
        }
    }
}

impl FrameDescriptionEntry {
    pub fn run(&self, pc: usize) -> Result<ExecutionResult, UnwindError> {
        let base = ExecutionResult::new(
            Register::from_unwind_regnum(self.cie.ret_addr_column as usize)?,
            self.pc.start,
        );
        self.cie
            .initial_instructions
            .run_from_result(usize::MAX, base, &self.cie)
            .and_then(|init| self.instructions.run_from_result(pc, init, &self.cie))
    }
}
