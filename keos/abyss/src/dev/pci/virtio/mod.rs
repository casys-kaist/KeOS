//! VirtIo device drivers.
//!
//! <https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-100001>

pub mod block;
pub mod pci;
mod tys;
pub mod virt_queue;

use crate::spinlock::{SpinLock, SpinLockGuard};
use core::sync::atomic::Ordering;
pub use pci::PciTransport;
pub use tys::*;
pub use virt_queue::VirtQueue;

#[derive(Debug)]
pub struct VirtIOError;

pub trait VirtIoDeviceFeature {
    fn bits(&self) -> u64;
    fn from_bits_truncate(val: u64) -> Self;
}

pub struct VirtIoDevice<V: Send + Sync, const MAX_QUEUE: usize> {
    virtqs: [SpinLock<VirtQueue>; MAX_QUEUE],
    pub transport: PciTransport<V>,
}

impl<V: Send + Sync, const MAX_QUEUE: usize> VirtIoDevice<V, MAX_QUEUE> {
    #[inline]
    pub fn from_transport(transport: PciTransport<V>) -> Self {
        Self {
            transport,
            virtqs: [0; MAX_QUEUE].map(|_| SpinLock::new(VirtQueue::empty())),
        }
    }

    #[inline]
    pub fn init<F, FN>(
        &self,
        common_feature_mask: VirtIoFeaturesCommon,
        device_feature_mask: F,
        device_init: FN,
    ) -> Result<(), VirtIOError>
    where
        F: VirtIoDeviceFeature,
        FN: FnOnce(&Self, VirtIoFeaturesCommon, F) -> Result<(), VirtIOError>,
    {
        // 3.1.1 Driver Requirements: Device Initialization
        let status = self.transport.common.device_status();
        // reset device.
        status.write(Status::empty());

        // Wait until read returns zero.
        loop {
            match status.read() {
                status if status.is_empty() => break,
                status if status.contains(Status::FAILED) => return Err(VirtIOError),
                _ => (),
            }
        }
        // Step 2-3.
        status.write(Status::ACKNOWLEDGE | Status::DRIVER);
        // Step 4. Ack features
        let features = self.transport.common.get_device_features();
        let common_features = VirtIoFeaturesCommon::from_bits_truncate(features)
            & common_feature_mask
            | VirtIoFeaturesCommon::VERSION_1;
        let device_features = F::from_bits_truncate(features & device_feature_mask.bits());
        self.transport.feat.store(
            common_features.bits() | device_features.bits(),
            Ordering::Relaxed,
        );
        self.transport
            .common
            .set_features(common_features.bits() | device_features.bits());
        // Step 5.
        status.write(status.read() | Status::FEATURES_OK);
        // Step 6.
        if !status.read().contains(Status::FEATURES_OK) {
            return Err(VirtIOError);
        }
        // Step 7. Device-specific setups
        device_init(self, common_features, device_features)?;
        // Step 8.
        status.write(status.read() | Status::DRIVER_OK);
        Ok(())
    }

    #[inline]
    pub fn configure_queue<F, R>(&self, qid: u16, f: F) -> R
    where
        F: FnOnce(QueueScope<V, MAX_QUEUE>) -> R,
    {
        assert!(qid < MAX_QUEUE as u16);
        self.transport.select_queue(qid);
        f(QueueScope { qid, dev: self })
    }

    #[inline]
    pub fn get_queue(&self, qid: u16) -> Option<SpinLockGuard<'_, VirtQueue>> {
        self.virtqs.get(qid as usize).map(|n| n.lock())
    }
}

pub struct QueueScope<'a, V: Send + Sync, const MAX_QUEUE: usize> {
    qid: u16,
    dev: &'a VirtIoDevice<V, MAX_QUEUE>,
}

impl<'dev, 'qs, V: Send + Sync, const MAX_QUEUE: usize> QueueScope<'dev, V, MAX_QUEUE> {
    #[inline]
    pub fn queue_builder(&'qs self) -> QueueBuilder<'dev, 'qs, V, MAX_QUEUE> {
        QueueBuilder {
            scope: self,
            size: 0,
        }
    }

    #[inline]
    pub fn queue_size(&self) -> u16 {
        self.dev.transport.queue_size()
    }
}

pub struct QueueBuilder<'a, 'b, V: Send + Sync, const MAX_QUEUE: usize> {
    scope: &'b QueueScope<'a, V, MAX_QUEUE>,
    size: u16,
}

impl<V: Send + Sync, const MAX_QUEUE: usize> QueueBuilder<'_, '_, V, MAX_QUEUE> {
    #[inline]
    pub fn set_size(mut self, size: u16) -> Result<Self, VirtIOError> {
        if size.is_power_of_two() {
            self.size = size;
            Ok(self)
        } else {
            Err(VirtIOError)
        }
    }

    #[inline]
    pub fn register(self) {
        let mut guard = self.scope.dev.virtqs[self.scope.qid as usize].lock();
        let kick = self.scope.dev.transport.get_kick();
        *guard = VirtQueue::new(
            self.size,
            self.scope.qid,
            VirtIoFeaturesCommon::from_bits_truncate(
                self.scope.dev.transport.get_driver_features(),
            )
            .contains(VirtIoFeaturesCommon::RING_EVENT_IDX),
            kick,
        );
        unsafe {
            self.scope.dev.transport.register_virtqueue(&guard);
        }
        guard.unlock();
    }
}
