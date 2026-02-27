//! Devices.

#[macro_use]
pub mod mmio;
pub mod pci;
pub mod x86_64;

use alloc::boxed::Box;

#[derive(Debug)]
#[allow(dead_code)]
pub struct DeviceError(&'static str);

// Even though, there could be more than 4 virtio dev, just set maxium device
// number to 4. Slot 0: Kernel image. For debugging purpose.
// Slot 1: Filesystem disk 1.
static mut BLOCK_DEVS: [Option<Box<dyn BlockOps>>; 4] = [None, None, None, None];

/// Get block device.
///
/// - Slot 0: Kernel image. For debugging purpose.
/// - Slot 1: Filesystem disk 1.
pub fn get_bdev(slot_idx: usize) -> Option<&'static dyn BlockOps> {
    unsafe { BLOCK_DEVS.get(slot_idx).and_then(|n| n.as_deref()) }
}

/// Sector, an access granuality for the disk.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Sector(pub usize);

impl Sector {
    /// Get offset that represented by the sector.
    #[inline]
    pub fn into_offset(self) -> usize {
        self.0 * 512
    }

    /// Cast into usize.
    #[inline]
    pub fn into_usize(self) -> usize {
        self.0
    }
}

impl core::ops::Add<usize> for Sector {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}

pub trait BlockOps {
    /// Initialize the block device.
    fn init(&self) -> bool;
    /// Get total block count of this device.
    fn block_cnt(&self) -> usize;
    /// get block size of this device.
    fn block_size(&self) -> usize;
    /// Read 512 bytes from disk starting from sector.
    fn read(&self, sector: Sector, buf: &mut [u8; 512]) -> bool;
    /// Write 512 bytes to disk starting from sector.
    fn write(&self, sector: Sector, buf: &[u8; 512]) -> bool;
    #[doc(hidden)]
    fn read_block_many(&self, _offset: usize, _buf: &mut [u8]) -> bool {
        unimplemented!()
    }
}
