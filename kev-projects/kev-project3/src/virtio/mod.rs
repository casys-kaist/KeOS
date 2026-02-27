//! Simple Virtio block device
pub mod virt_queue;

/// The header of the virtio device.
#[repr(C)]
#[derive(Debug, Default)]
pub struct VirtIoMmioHeader {
    /// Status of the device
    pub status: u32,
    /// Size of the virtqueue
    pub queue_size: u32,
    /// Upper 32bit of the virtqueue physical address
    pub queue_addr_hi: u32,
    /// Lower 32bit of the virtqueue physical address
    pub queue_addr_lo: u32,
    /// Queue Head
    ///
    /// Driver update the tail of the queue. Device must not update the field.
    pub queue_head: u32,
    /// Queue tail
    ///
    /// Device update the tail of the queue. Driver must not update the field.
    pub queue_tail: u32,
}

impl VirtIoMmioHeader {
    pub fn new() -> Self {
        VirtIoMmioHeader {
            status: 0,
            queue_size: 0,
            queue_addr_hi: 0,
            queue_addr_lo: 0,
            queue_head: 0,
            queue_tail: 0,
        }
    }
}

/// A possible status of sVirtIO device.
#[derive(Debug, PartialEq)]
#[repr(u32)]
pub enum VirtIoStatus {
    /// A Magic value.
    MAGIC = 0x74726976,
    /// Device is ok.
    DRIVEROK = 0,
    /// Device is ready.
    READY = 1,
    /// Reset the device.
    RESET = 2,
}

impl TryFrom<u32> for VirtIoStatus {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            x if x == VirtIoStatus::MAGIC as u32 => Ok(VirtIoStatus::MAGIC),
            x if x == VirtIoStatus::DRIVEROK as u32 => Ok(VirtIoStatus::DRIVEROK),
            x if x == VirtIoStatus::READY as u32 => Ok(VirtIoStatus::READY),
            x if x == VirtIoStatus::RESET as u32 => Ok(VirtIoStatus::RESET),
            _ => Err(()),
        }
    }
}
