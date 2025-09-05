use crate::addressing::Pa;
use crate::x86_64::pio::Pio;

/// BAR on IO Space.
#[derive(Debug)]
pub struct IoSpace {
    pub(crate) base: u32,
    pub(crate) length: u32,
}

impl IoSpace {
    /// Access offset on the IO Space.
    #[cfg(target_arch = "x86_64")]
    pub fn offset<const IDX: u32>(&self) -> Option<Pio> {
        IDX.checked_add(4).and_then(|i| {
            if i <= self.length + 1 {
                Some(Pio::new((self.base + IDX) as u16))
            } else {
                None
            }
        })
    }
}

/// BAR on Memory Space.
#[derive(Debug)]
pub struct MemorySpace {
    pub(crate) base: Pa,
    pub(crate) length: usize,
    pub(crate) _prefetchable: bool,
}

impl MemorySpace {
    pub fn all(&self) -> crate::dev::mmio::MmioArea {
        unsafe { crate::dev::mmio::MmioArea::new(self.base..(self.base + self.length)) }
    }

    pub fn try_split_mmio_range(
        &self,
        offset: usize,
        length: usize,
    ) -> Option<crate::dev::mmio::MmioArea> {
        if offset < self.length
            && offset
                .checked_add(length)
                .map(|limit| limit <= self.length)
                .unwrap_or(false)
        {
            unsafe {
                Some(crate::dev::mmio::MmioArea::new(
                    (self.base + offset)..(self.base + offset + length),
                ))
            }
        } else {
            None
        }
    }
}
/// Enumeration of Base Address Register (BAR) types.
#[derive(Debug)]
pub enum Bar {
    /// Bar on the Memory Space.
    // Bit3: Prefetchable, Bit 2-1: Type
    MemorySpace(MemorySpace),
    /// Bar on the IO Space.
    IoSpace(IoSpace),
}

impl Bar {
    pub fn try_get_memory_bar(self) -> Option<MemorySpace> {
        if let Self::MemorySpace(memory_space) = self {
            Some(memory_space)
        } else {
            None
        }
    }

    pub fn try_get_io_bar(self) -> Option<IoSpace> {
        if let Self::IoSpace(io_space) = self {
            Some(io_space)
        } else {
            None
        }
    }
}
