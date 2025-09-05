//! VirtIO Block device driver.
//!
//! The virtio block device is a simple virtual block device (ie. disk). Read
//! and write requests (and other exotic requests) are placed in the queue, and
//! serviced (probably out of order) by the device except where noted.

// mod adaptor;
mod tys;

use crate::dev::pci::PciDeviceHeader;
use crate::dev::pci::virtio::{PciTransport, VirtIoDevice, VirtIoFeaturesCommon};
use tys::*;

use super::VirtIOError;

mmio! {
    /// Device configuration layout
    ///
    /// The capacity of the device (expressed in 512-byte sectors) is always
    /// present. The availability of the others all depend on various feature bits
    /// as indicated above. The parameters in the configuration space of the device
    /// max_discard_sectors discard_sector_alignment are expressed in 512-byte units
    /// if the VIRTIO_BLK_F_DISCARD feature bit is negotiated. The
    /// max_write_zeroes_sectors is expressed in 512-byte units if the
    /// VIRTIO_BLK_F_WRITE_ZEROES feature bit is negotiated.
    pub VirtIoBlockCfg:
        capacity @ 0 => RW, u64;
        size_max @ 8 => RW, u32;
        seg_max @ 12 => RW, u32;

        // geometry
        geometry_cylinders @ 16 => RW, u16;
        geometry_heads @ 18 => RW, u8;
        geometry_sectors @ 19 => RW, u8;

        blk_size @ 20 => RW, u16;

        // topology
        /// # of blocks per physical block (log2)
        topology_physical_block_exp @ 22 => RW, u8;
        /// offset of first aligned logical block
        topology_alignment_offset @ 23 => RW, u8;
        /// suggested minimum I/O size in blocks
        topology_min_io_size @ 24 => RW, u16;
        /// optimal (suggested maximum) I/O size in blocks
        topology_opt_io_size @ 26 => RW, u32;
        writeback @ 28 => RW, u8;
        max_discard_sectors @ 32 => RW, u32;
        max_discard_seg @ 36 => RW, u32;
        discard_sector_alignment @ 40 => RW, u32;
        max_write_zeros_sectors @ 44 => RW, u32;
        max_write_zeros_seg @ 48 => RW, u32;
        write_zeros_may_unmap @ 52 => RW, u8;
}

pub struct VirtIoBlock {
    dev: VirtIoDevice<VirtIoBlockCfg, 1>,
    // Cached property.
    block_size: usize,
    block_count: usize,
}

impl VirtIoBlock {
    pub fn from_pci(pci: PciDeviceHeader) -> Result<Self, VirtIOError> {
        if let PciDeviceHeader::Type0(pci) = pci {
            let conf = PciTransport::new(pci, VirtIoBlockCfg::new_from_mmio_area);
            let (block_size, block_count) = (
                conf.blk_size().read() as usize,
                conf.capacity().read() as usize,
            );

            Ok(Self {
                dev: VirtIoDevice::from_transport(conf),
                block_size,
                block_count,
            })
        } else {
            Err(VirtIOError)
        }
    }

    pub fn init(&self) -> Result<(), VirtIOError> {
        self.dev.init(
            VirtIoFeaturesCommon::empty(),
            VirtIoFeaturesBlock::all(),
            |dev, _comm_feat, _dev_feat| {
                // 5.2 Block Device.
                // 4.1.5.1.3 Virtqueue Configuration
                dev.configure_queue(0, |scope| {
                    let queue_max_size = scope.queue_size();
                    scope.queue_builder().set_size(queue_max_size)?.register();
                    Ok(())
                })
            },
        )
    }

    /// Get total block count of this device.
    #[inline]
    pub fn block_cnt(&self) -> usize {
        self.block_count
    }

    /// get block size of this device.
    #[inline]
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Flush read bio request to the disk.
    pub fn read_bios(
        &self,
        bios: &mut dyn Iterator<Item = (usize, &mut [u8])>,
    ) -> Result<(), VirtIOError> {
        let (mut virtq, mut req, mut resp) = (
            self.dev.get_queue(0).unwrap(),
            VirtIoBlockReq {
                type_: VirtIoBlockType::In,
                sector: 0,
                __reserved: 0,
            },
            VirtIoBlockResp::default(),
        );

        let mut bios = bios.peekable();
        while let Some((ofs, buf)) = bios.next() {
            let ofs_sector = if ofs % self.block_size == 0 && buf.len() % self.block_size == 0 {
                ofs / self.block_size
            } else {
                return Err(VirtIOError);
            };
            let mut remain = virtq.size() - 3;
            let mut tx = virtq.sgl_builder();
            let mut expected = ofs + buf.len();
            req.sector = ofs_sector as u64;
            tx.push(&req);
            tx.push_mut(buf);

            while let Some((ofs, _)) = bios.peek() {
                if remain != 0 && *ofs == expected {
                    let (_, buf) = bios.next().unwrap();
                    expected += buf.len();
                    remain -= 1;
                    tx.push_mut(buf);
                } else {
                    break;
                }
            }
            tx.push_mut(&mut resp);
            tx.finish();
            if !matches!(resp, VirtIoBlockResp::Ok) {
                virtq.unlock();
                return Err(VirtIOError);
            }
        }
        virtq.unlock();
        Ok(())
    }

    /// Flush write bio request to the disk.
    pub fn write_bios(
        &self,
        bios: &mut dyn Iterator<Item = (usize, &[u8])>,
    ) -> Result<(), VirtIOError> {
        let (mut virtq, mut req, mut resp) = (
            self.dev.get_queue(0).unwrap(),
            VirtIoBlockReq {
                type_: VirtIoBlockType::Out,
                sector: 0,
                __reserved: 0,
            },
            VirtIoBlockResp::default(),
        );

        let mut bios = bios.peekable();
        while let Some((ofs, buf)) = bios.next() {
            let ofs_sector = if ofs % self.block_size == 0 && buf.len() % self.block_size == 0 {
                ofs / self.block_size
            } else {
                return Err(VirtIOError);
            };
            let mut remain = virtq.size() - 3;
            let mut tx = virtq.sgl_builder();
            let mut expected = ofs + buf.len();
            req.sector = ofs_sector as u64;
            tx.push(&req);
            tx.push(buf);

            while let Some((ofs, _)) = bios.peek() {
                if remain != 0 && *ofs == expected {
                    let (_, buf) = bios.next().unwrap();
                    expected += buf.len();
                    remain -= 1;
                    tx.push(buf);
                } else {
                    break;
                }
            }
            tx.push_mut(&mut resp);
            tx.finish();
            if !matches!(resp, VirtIoBlockResp::Ok) {
                virtq.unlock();
                return Err(VirtIOError);
            }
        }
        virtq.unlock();
        Ok(())
    }
}
