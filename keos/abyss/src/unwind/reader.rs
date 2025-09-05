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

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Format {
    /// Pointer size.
    Pointer,
    /// Unsigned value is encoded using the Little Endian Base 128 (LEB128) as
    /// defined by DWARF Debugging Information Format, Revision 2.0.0 (July 27,
    /// 1993).
    UnsignedData16,
    /// A 2 bytes unsigned value.
    UnsignedData2,
    /// A 4 bytes unsigned value.
    UnsignedData4,
    /// An 8 bytes unsigned value.
    UnsignedData8,
    /// Signed value is encoded using the Little Endian Base 128 (LEB128) as
    /// defined by DWARF Debugging Information Format, Revision 2.0.0 (July 27,
    /// 1993).
    SignedData16,
    /// A 2 bytes signed value.
    SignedData2,
    /// A 4 bytes signed value.
    SignedData4,
    /// An 8 bytes signed value.
    SignedData8,
    /// Unused
    Unused,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Application {
    /// Value is used with no modification. 0
    Absoulte,
    /// Value is reletive to the current program counter. 1
    PcRelative,
    /// Value is reletive to the beginning of the .eh_frame_hdr section. 2
    DataRelative,
}

#[derive(Debug, Clone, Copy)]
pub struct Encoding {
    pub format: Format,
    pub application: Application,
}

impl core::convert::From<u8> for Encoding {
    #[inline]
    fn from(s: u8) -> Self {
        let (format, app) = (s & 15, (s >> 4) & 7);
        Self {
            format: match format {
                0 => Format::Pointer,
                1 => Format::UnsignedData16,
                2 => Format::UnsignedData2,
                3 => Format::UnsignedData4,
                4 => Format::UnsignedData8,
                9 => Format::SignedData16,
                10 => Format::SignedData2,
                11 => Format::SignedData4,
                12 => Format::SignedData8,
                15 => Format::Unused,
                e => unreachable!("Corrupted EH frame header: {:?}", e),
            },
            application: match app {
                0 => Application::Absoulte,
                1 => Application::PcRelative,
                3 => Application::DataRelative,
                e => unreachable!("Corrupted EH frame header: {:?}", e),
            },
        }
    }
}

impl Encoding {
    #[inline]
    pub fn size(&self) -> usize {
        match self.format {
            Format::SignedData2 | Format::UnsignedData2 => 2,
            Format::SignedData4 | Format::UnsignedData4 => 4,
            Format::SignedData8 | Format::UnsignedData8 => 8,
            Format::SignedData16 | Format::UnsignedData16 => 16,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct Reader {
    pub start: usize,
    pub end: usize,
    pub pos: usize,
}

#[inline]
pub fn get_uleb128<F>(mut f: F) -> Option<usize>
where
    F: FnMut() -> Option<u8>,
{
    let (mut val, mut shift, mut ch) = (0, 0, 0x80);
    while ch & 0x80 == 0x80 {
        ch = f()?;
        val |= (ch as usize & 0x7f) << shift;
        shift += 7;
    }
    Some(val)
}

#[inline]
pub fn get_sleb128<F>(mut f: F) -> Option<isize>
where
    F: FnMut() -> Option<u8>,
{
    let (mut val, mut shift, mut ch) = (0, 0, 0x80);
    while ch & 0x80 == 0x80 {
        ch = f()?;
        val |= (ch as usize & 0x7f) << shift;
        shift += 7;
    }

    if shift < 8 * core::mem::size_of::<usize>() && (ch & 0x40) != 0 {
        Some((val | ((-1_isize as usize) << shift)) as isize)
    } else {
        Some(val as isize)
    }
}

/// Byte peekable object.
pub trait Peeker
where
    Self: Clone,
{
    fn read<T>(&self, pos: usize) -> Option<T>
    where
        T: Copy;
}

#[derive(Clone)]
pub struct DwarfReader<T>
where
    T: Peeker,
{
    pub(crate) start: usize,
    pub(crate) pos: usize,
    pub(crate) end: usize,
    peeker: T,
}

impl<T> DwarfReader<T>
where
    T: Peeker,
{
    pub fn from_peeker(start: usize, peeker: T) -> Self {
        Self {
            peeker,
            start,
            pos: 0,
            end: usize::MAX,
        }
    }

    pub fn reset(&mut self, start: usize) {
        self.start = start;
        self.pos = 0;
        self.end = usize::MAX;
    }

    #[inline]
    pub fn current(&self) -> usize {
        self.start + self.pos
    }

    #[inline]
    pub fn wheel(&mut self, p: isize) {
        self.pos += p as usize;
    }

    #[inline]
    pub fn set_end(&mut self, p: usize) -> bool {
        if self.end >= self.start + p {
            self.end = self.start + p;
            true
        } else {
            false
        }
    }

    pub fn end(&self) -> usize {
        self.end
    }

    #[inline]
    pub fn read<V: Copy>(&mut self) -> Option<V> {
        let size = core::mem::size_of::<V>();
        if self.pos + size <= self.end {
            let out = self.peeker.read(self.current());
            self.pos += size;
            out
        } else {
            None
        }
    }

    #[inline]
    pub fn read_uleb128(&mut self) -> Option<usize> {
        get_uleb128(|| self.read::<u8>())
    }

    #[inline]
    pub fn read_sleb128(&mut self) -> Option<isize> {
        get_sleb128(|| self.read::<u8>())
    }

    #[inline]
    pub fn read_with_encoding(&mut self, encoding: Encoding) -> Option<usize> {
        let base = match encoding.application {
            Application::Absoulte => 0,
            Application::PcRelative => self.current(),
            Application::DataRelative => self.start,
        };

        match encoding.format {
            Format::Pointer => Some(base.overflowing_add(self.read::<usize>()?).0),
            Format::UnsignedData16 => Some(base.overflowing_add(self.read_uleb128()?).0),
            Format::UnsignedData2 => Some(base.overflowing_add(self.read::<u16>()? as usize).0),
            Format::UnsignedData4 => Some(base.overflowing_add(self.read::<u32>()? as usize).0),
            Format::UnsignedData8 => Some(base.overflowing_add(self.read::<u64>()? as usize).0),
            Format::SignedData16 => Some(base.overflowing_add(self.read_sleb128()? as usize).0),
            Format::SignedData2 => Some(base.overflowing_add(self.read::<i16>()? as usize).0),
            Format::SignedData4 => Some(base.overflowing_add(self.read::<i32>()? as usize).0),
            Format::SignedData8 => Some(base.overflowing_add(self.read::<i64>()? as usize).0),
            Format::Unused => None,
        }
    }
}
