bitflags::bitflags! {
    pub struct VirtIoFeaturesBlock: u64 {
        /// Device supports request barriers.
        const BARRIER = 1 << 0;
        /// Maximum size of any single segment is in size_max.
        const SIZE_MAX = 1 << 1;
        /// Maximum number of segments in a request is in seg_max.
        const SEG_MAX = 1 << 2;
        /// Disk-style geometry specified in geometry.
        const GEOMETRY = 1 << 4;
        /// Device is read-only.
        const RO = 1 << 5;
        /// Block size of disk is in blk_size.
        const BLK_SIZE = 1 << 6;
        /// Device supports scsi packet commands.
        const SCSI = 1 << 7;
        /// Cache flush command support.
        const FLUSH = 1 << 9;
        /// Cache flush command support.
        const TOPOLOGY = 1 << 10;
        /// Device can toggle its cache between writeback and writethrough modes.
        const CONFIG_WCE = 1 << 11;
        /// Device can support discard command, maximum discard sectors size in
        /// max_discard_sectors and maximum discard segment number in
        /// max_discard_seg.
        const DISCARD = 1 << 13;
        /// Device can support write zeroes command, maximum write zeroes
        /// sectors size in max_write_zeroes_sectors and maximum write zeroes
        /// segment number in max_write_zeroes_seg.
        const WRITE_ZEROS = 1 << 14;
    }
}

impl crate::dev::pci::virtio::VirtIoDeviceFeature for VirtIoFeaturesBlock {
    fn from_bits_truncate(val: u64) -> Self {
        Self::from_bits_truncate(val)
    }
    fn bits(&self) -> u64 {
        self.bits()
    }
}

#[derive(Clone, Copy)]
#[repr(u32)]
#[allow(dead_code)]
pub enum VirtIoBlockType {
    /// Read.
    In = 0,
    /// Write.
    Out = 1,
    /// Flush.
    Flush = 4,
    /// Discard.
    Discard = 11,
    /// Write Zeros.
    WriteZeros = 13,
}

#[repr(C)]
pub struct VirtIoBlockReq {
    pub type_: VirtIoBlockType,
    pub __reserved: u32,
    pub sector: u64,
}

impl Default for VirtIoBlockReq {
    fn default() -> Self {
        Self {
            type_: VirtIoBlockType::In,
            __reserved: 0,
            sector: 0,
        }
    }
}

#[repr(C)]
#[allow(dead_code)]
pub struct VirtIoBlockDiscardWriteZeros {
    pub sector: u64,
    pub num_sectors: u32,
    pub flags: u32,
}

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum VirtIoBlockResp {
    Ok = 0,
    IoErr = 1,
    Unsupported = 2,
}

impl Default for VirtIoBlockResp {
    fn default() -> Self {
        Self::IoErr
    }
}
