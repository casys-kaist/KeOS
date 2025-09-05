//! Devices.

#[macro_use]
pub mod mmio;
pub mod pci;
pub mod x86_64;

use pci::virtio::block::VirtIoBlock;

#[derive(Debug)]
#[allow(dead_code)]
pub struct DeviceError(&'static str);

// Even though, there could be more than 4 virtio dev, just set maxium device
// number to 4. Slot 0: Kernel image. For debugging purpose.
// Slot 1: Filesystem disk 1.
static mut BLOCK_DEVS: [Option<VirtIoBlock>; 4] = [None, None, None, None];

/// Get block device.
///
/// - Slot 0: Kernel image. For debugging purpose.
/// - Slot 1: Filesystem disk 1.
pub fn get_bdev(slot_idx: usize) -> Option<&'static VirtIoBlock> {
    unsafe { BLOCK_DEVS.get(slot_idx).and_then(|n| n.as_ref()) }
}
