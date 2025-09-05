use super::{Region, Regions};
use crate::addressing::Pa;

#[derive(Debug)]
pub struct MemoryMap<'a> {
    pub(crate) _version: u32,
    pub(crate) stride: u32,
    pub(crate) total_size: usize,
    pub(crate) entries: &'a [u8; 0],
}

impl MemoryMap<'_> {
    pub(crate) fn iter(&self) -> MemoryMapIter<'_> {
        MemoryMapIter {
            memory_map: self,
            pos: 0,
        }
    }
}

// The E820 entries.
#[repr(C, packed)]
pub(crate) struct MemoryMapEntry {
    pub(crate) base_addr: u64,
    pub(crate) length: u64,
    pub(crate) ty: u32,
}

impl From<&MemoryMapEntry> for Region {
    fn from(mm: &MemoryMapEntry) -> Region {
        let (start, size) = (Pa::new(mm.base_addr as usize).unwrap(), mm.length as usize);
        Region {
            addr: start..start + size,
            usable: mm.ty == 1,
        }
    }
}

impl From<MemoryMap<'_>> for Regions {
    #[allow(clippy::suspicious_operation_groupings)]
    fn from(mm_info: MemoryMap) -> Regions {
        const NULL_REGION: Region = Region {
            addr: Pa::ZERO..Pa::ZERO,
            usable: false,
        };

        let mut regions = [NULL_REGION; 64];
        let entries = mm_info.iter();
        let entry_counts = entries.len();
        for (idx, en) in entries.enumerate() {
            regions[idx] = Region::from(en)
        }

        // sort by addr.
        regions[..entry_counts].sort_unstable_by(|a, b| a.addr.start.cmp(&b.addr.start));

        let mut committed = 0;
        (1..entry_counts).for_each(|idx| {
            // If mergable, extend end.
            if regions[committed].addr.end == regions[idx].addr.start
                && regions[committed].usable == regions[idx].usable
            {
                regions[committed].addr.end = regions[idx].addr.end;
            } else {
                // Unmergable.
                committed += 1;
                if committed != idx {
                    regions[committed] = regions[idx].clone();
                }
            }
        });

        Regions {
            regions,
            size: committed + 1,
        }
    }
}

/// Memory map iterator
pub(crate) struct MemoryMapIter<'a> {
    memory_map: &'a MemoryMap<'a>,
    pos: usize,
}

impl<'a> core::iter::Iterator for MemoryMapIter<'a> {
    type Item = &'a MemoryMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.memory_map.total_size < self.pos {
            return None;
        }
        let o = unsafe {
            ((self.memory_map.entries.as_ptr() as usize + self.pos) as *const MemoryMapEntry)
                .as_ref()
        }
        .unwrap();
        self.pos += self.memory_map.stride as usize;
        Some(o)
    }
}

impl core::iter::ExactSizeIterator for MemoryMapIter<'_> {
    fn len(&self) -> usize {
        self.memory_map.total_size / (self.memory_map.stride as usize)
    }
}

/// Mutiboot2 info
#[repr(C)]
#[derive(Debug)]
pub(crate) struct MultiBootInfo2 {
    pub(crate) total_size: u32,
    pub(crate) _rev: u32,
    pub(crate) fields: [u8; 0],
}

#[repr(C)]
struct Header {
    ty: u32,
    size: u32,
    o: [u8; 0],
}

impl Header {
    fn read_at<T>(&self, p: usize) -> T
    where
        T: Copy,
    {
        unsafe {
            *((self.o.as_ptr() as usize + p) as *const T)
                .as_ref()
                .unwrap()
        }
    }
}

impl MultiBootInfo2 {
    pub(crate) fn get_cmdline(&self) -> Option<&'static [u8]> {
        let mut pos = 0;

        while pos + 8 <= self.total_size as usize {
            let en = unsafe {
                ((self.fields.as_ptr() as usize + pos) as *const Header)
                    .as_ref()
                    .unwrap()
            };
            pos += ((en.size + 7) & !7) as usize;
            match en.ty {
                0 => {
                    return None;
                }
                1 => unsafe {
                    return Some(core::slice::from_raw_parts(
                        en.o.as_ptr(),
                        en.size as usize - 8,
                    ));
                },
                _ => (),
            }
        }
        None
    }

    pub(crate) fn get_memory_map(&self) -> Option<MemoryMap<'_>> {
        let mut pos = 0;

        while pos + 8 <= self.total_size as usize {
            let en = unsafe {
                ((self.fields.as_ptr() as usize + pos) as *const Header)
                    .as_ref()
                    .unwrap()
            };
            pos += ((en.size + 7) & !7) as usize;
            match en.ty {
                0 => {
                    return None;
                }
                6 => {
                    return Some(MemoryMap {
                        stride: en.read_at(0),
                        _version: en.read_at(4),
                        total_size: en.size as usize - 16,
                        entries: unsafe {
                            ((en.o.as_ptr() as usize + 8) as *const [u8; 0]).as_ref()
                        }
                        .unwrap(),
                    });
                }
                _ => (),
            }
        }
        None
    }
}
