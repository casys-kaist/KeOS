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

use super::IsrCfg;
use super::virt_queue::{Kick, VirtQueue};
use crate::addressing::Kva;
use crate::dev::mmio::MmioArea;
use crate::dev::pci::{self, Capability};
use core::sync::atomic::{AtomicU64, Ordering};

#[repr(u8)]
#[derive(Debug, Eq, PartialEq)]
pub enum PciCapabilityType {
    /// Common configuration
    CommonCfg = 1,
    /// Notifications
    NotifyCfg = 2,
    /// ISR Status
    IsrCfg = 3,
    /// Device specific configuration
    DeviceCfg = 4,
    /// PCI configuration access
    PciCfg = 5,
    Unknown,
}

struct VirtIoCapability<'a> {
    cap: Capability<'a, 0>,
}

impl<'a> VirtIoCapability<'a> {
    const VENDOR_VIRTIO: u8 = 9;
    const TYPE_OFFSET: u8 = 3;
    const BAR_OFFSET: u8 = 4;
    const OFFSET_OFFSET: u8 = 8;
    const LENGTH_OFFSET: u8 = 12;

    #[inline]
    fn try_from(cap: Capability<'a, 0>) -> Option<Self> {
        if cap.vendor() == Self::VENDOR_VIRTIO {
            Some(VirtIoCapability { cap })
        } else {
            None
        }
    }

    #[inline]
    fn ty(&self) -> PciCapabilityType {
        match self.cap.offset(Self::TYPE_OFFSET).read_u8() {
            1 => PciCapabilityType::CommonCfg,
            2 => PciCapabilityType::NotifyCfg,
            3 => PciCapabilityType::IsrCfg,
            4 => PciCapabilityType::DeviceCfg,
            5 => PciCapabilityType::PciCfg,
            _ => PciCapabilityType::Unknown,
        }
    }

    #[inline]
    fn bar(&self) -> u8 {
        self.cap.offset(Self::BAR_OFFSET).read_u8()
    }

    #[inline]
    fn offset(&self) -> usize {
        self.cap.offset(Self::OFFSET_OFFSET).read_u32() as usize
    }

    #[inline]
    fn length(&self) -> usize {
        self.cap.offset(Self::LENGTH_OFFSET).read_u32() as usize
    }
}

mmio! {
    /// 4.1.4.3 Common configuration structure layout
    pub VirtIoPciCommonCfg:
        /// The driver uses this to select which feature bits device_feature shows. Value 0x0 selects Feature Bits 0 to 31, 0x1 selects Feature Bits 32 to 63, etc.
        device_features_select @ 0 => W, u32;
        /// The device uses this to report which feature bits it is offering to the driver: the driver writes to device_feature_select to select which feature bits are presented.
        device_features @ 4 => R, u32;
        /// The driver uses this to select which feature bits driver_feature shows. Value 0x0 selects Feature Bits 0 to 31, 0x1 selects Feature Bits 32 to 63, etc.
        driver_features_select @ 8 => W, u32;
        /// The driver writes this to accept feature bits offered by the device. Driver Feature Bits selected by driver_feature_select.
        driver_features @ 12 => W, u32;

        /* About the whole device. */
        /// The driver sets the Configuration Vector for MSI-X.
        msix_config @ 16 => RW, u16;
        /// The device specifies the maximum number of virtqueues supported here.
        num_queues @ 18 => R, u16;
        /// The driver writes the device status here (see 2.1). Writing 0 into this field resets the device.
        device_status @ 20 => RW, crate::dev::pci::virtio::Status;
        /// Configuration atomicity value. The device changes this every time the configuration noticeably changes.
        config_generation @ 21 => R, u8;

        /* About a specific virtqueue. */
        /// Queue Select. The driver selects which virtqueue the following fields refer to.
        queue_select @ 22 => RW, u16;
        /// Queue Size. On reset, specifies the maximum queue size supported by the device. This can be modified by the driver to reduce memory requirements. A 0 means the queue is unavailable.
        queue_size @ 24 => RW, u16;
        /// The driver uses this to specify the queue vector for MSI-X.
        queue_msix_vector @ 26 => RW, u16;
        /// The driver uses this to selectively prevent the device from executing requests from this virtqueue. 1 - enabled; 0 - disabled.
        queue_enable @ 28 => RW, u16;
        /// The driver reads this to calculate the offset from start of Notification structure at which this virtqueue is located. Note: this is not an offset in bytes. See 4.1.4.4 below.
        queue_notify_off @ 30 => R, u16;
        /// The driver writes the physical address of Descriptor Area here. See section 2.5.
        queue_desc @ 32 => RW, u64;
        /// The driver writes the physical address of Driver Area here. See section 2.5.
        queue_driver @ 40 => RW, u64;
        /// The driver writes the physical address of Device Area here. See section 2.5.
        queue_device @ 48 => RW, u64;
}

impl VirtIoPciCommonCfg {
    #[inline]
    pub fn get_device_features(&self) -> u64 {
        self.device_features_select().write(0);
        let lo = self.device_features().read();
        self.device_features_select().write(1);
        let hi = self.device_features().read();
        ((hi as u64) << 32) | (lo as u64)
    }

    #[inline]
    pub fn set_features(&self, features: u64) {
        let (lo, hi) = (features as u32, (features >> 32) as u32);
        self.driver_features_select().write(0);
        self.driver_features().write(lo);
        self.driver_features_select().write(1);
        self.driver_features().write(hi);
    }
}

mmio! {
    /// 4.1.4.5 ISR status capability
    pub VirtIoIsrCfg:
        config @ 0 => RW, IsrCfg;
}

pub struct NotifyCfgTriple {
    memory_space: pci::MemorySpace,
    offset: usize,
    mult: usize,
}

pub fn try_get_configurations(
    pci: pci::PciHeader<0>,
) -> Option<(VirtIoPciCommonCfg, MmioArea, VirtIoIsrCfg, NotifyCfgTriple)> {
    let mut virtio_common_cfg = None;
    let mut virtio_device_cfg = None;
    let mut virtio_isr_cfg = None;
    let mut virtio_notify_cfg = None;

    for cap in pci.capabilities() {
        if let Some(cap) = VirtIoCapability::try_from(cap) {
            let (bar, offset, length) = (cap.bar(), cap.offset(), cap.length());
            match cap.ty() {
                PciCapabilityType::CommonCfg => {
                    virtio_common_cfg = pci
                        .bar(bar)
                        .and_then(|bar| bar.try_get_memory_bar())
                        .and_then(|memory_bar| memory_bar.try_split_mmio_range(offset, length))
                        .map(VirtIoPciCommonCfg::new_from_mmio_area)
                }
                PciCapabilityType::IsrCfg => {
                    virtio_isr_cfg = pci
                        .bar(bar)
                        .and_then(|bar| bar.try_get_memory_bar())
                        .and_then(|memory_bar| memory_bar.try_split_mmio_range(offset, length))
                        .map(VirtIoIsrCfg::new_from_mmio_area);
                }
                PciCapabilityType::DeviceCfg => {
                    virtio_device_cfg = pci
                        .bar(bar)
                        .and_then(|bar| bar.try_get_memory_bar())
                        .and_then(|memory_bar| memory_bar.try_split_mmio_range(offset, length))
                }
                PciCapabilityType::NotifyCfg => {
                    virtio_notify_cfg = Some(cap);
                }
                _ => (),
            }
        }
    }
    match (
        virtio_common_cfg,
        virtio_device_cfg,
        virtio_isr_cfg,
        virtio_notify_cfg,
    ) {
        (
            Some(virtio_common_cfg),
            Some(virtio_device_cfg),
            Some(virtio_isr_cfg),
            Some(virtio_notify_cfg),
        ) => {
            // calculate notify_cfg.
            let (bar, offset, notify_off_multiplier) = (
                virtio_notify_cfg.bar(),
                virtio_notify_cfg.offset(),
                virtio_notify_cfg.cap.offset(16).read_u32() as usize,
            );
            pci.bar(bar)
                .and_then(|bar| bar.try_get_memory_bar())
                .map(|memory_bar| {
                    (
                        virtio_common_cfg,
                        virtio_device_cfg,
                        virtio_isr_cfg,
                        NotifyCfgTriple {
                            memory_space: memory_bar,
                            offset,
                            mult: notify_off_multiplier,
                        },
                    )
                })
        }
        _ => None,
    }
}

pub struct PciTransport<V: Send + Sync> {
    pub _pci: pci::PciHeader<0>,
    pub common: VirtIoPciCommonCfg,
    pub isr: VirtIoIsrCfg,
    pub notify: NotifyCfgTriple,
    pub private: V,
    pub feat: AtomicU64,
}

impl<V: Send + Sync> PciTransport<V> {
    pub fn new<F>(pci: pci::PciHeader<0>, init_priv: F) -> Self
    where
        F: Fn(MmioArea) -> V,
    {
        let (common, mmio, isr, notify) =
            try_get_configurations(pci).expect("Not a valid virtio device");
        Self {
            _pci: pci,
            common,
            isr,
            notify,
            private: init_priv(mmio),
            feat: AtomicU64::new(0),
        }
    }

    pub unsafe fn register_virtqueue(&self, virtq: &VirtQueue) {
        let VirtQueue {
            desc, avail, used, ..
        } = virtq;

        self.common.queue_size().write(u16::to_le(virtq.size()));
        self.common.queue_desc().write(u64::to_le(
            Kva::new(desc.inner() as *const _ as usize)
                .unwrap()
                .into_pa()
                .into_usize() as u64,
        ));
        self.common.queue_driver().write(u64::to_le(
            Kva::new(avail.inner() as *const _ as usize)
                .unwrap()
                .into_pa()
                .into_usize() as u64,
        ));
        self.common.queue_device().write(u64::to_le(
            Kva::new(used.inner() as *const _ as usize)
                .unwrap()
                .into_pa()
                .into_usize() as u64,
        ));
        self.common.queue_enable().write(u16::to_le(1));
    }

    pub fn get_kick(&self) -> Kick {
        let NotifyCfgTriple {
            memory_space,
            offset,
            mult,
        } = &self.notify;
        let queue_notify_off = self.common.queue_notify_off().read() as usize;
        Kick::Pci(
            VirtIoNotifyCfg::new_from_mmio_area(
                memory_space
                    .try_split_mmio_range(offset + queue_notify_off * mult, 2)
                    .unwrap(),
            )
            .v(),
        )
    }

    pub fn select_queue(&self, idx: u16) {
        self.common.queue_select().write(idx)
    }

    pub fn queue_size(&self) -> u16 {
        self.common.queue_size().read()
    }

    pub fn get_driver_features(&self) -> u64 {
        self.feat.load(Ordering::Relaxed)
    }
}

impl<V: Send + Sync> core::ops::Deref for PciTransport<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.private
    }
}

mmio! {
    /// 2.7.23 Driver notifications
    pub VirtIoNotifyCfg:
        /// 16: vqn
        /// VQ number to be notified.
        /// 15: next_off
        /// Offset within the ring where the next available ring entry will be written. When VIRTIO_F_RING_PACKED has not been negotiated this refers to the 15 least significant bits of the available index. When VIRTIO_F_RING_PACKED has been negotiated this refers to the offset (in units of descriptor entries) within the descriptor ring where the next available descriptor will be written.
        /// next_wrap: 1
        /// Wrap Counter. With VIRTIO_F_RING_PACKED this is the wrap counter referring to the next available descriptor. Without VIRTIO_F_RING_PACKED this is the most significant bit (bit 15) of the available index.
        v @ 0 => RW, u16;
}
