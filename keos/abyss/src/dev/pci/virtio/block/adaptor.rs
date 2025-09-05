use crate::pci::PciDeviceHeader;
use crate::{VirtIoBlock, VirtIoBlockCfg, VirtIoBlockReq, VirtIoBlockResp, VirtIoBlockType};
use core::sync::atomic::Ordering;
use virtio_common::transport::{Mmio, Pci};
use virtio_common::VirtIoTransport;

impl<T: VirtIoTransport<Personality = VirtIoBlockCfg> + 'static> BlockOps for VirtIoBlock<T> where
    Self: StateManager
{
}

impl<T> DeviceOps for VirtIoBlock<T>
where
    T: VirtIoTransport<Personality = VirtIoBlockCfg>,
    T: 'static,
{
    type InitAux = ();

    // 4.1.4.8 Legacy Interfaces: A Note on PCI Device Layout
    fn realize(&mut self, _: Self::InitAux) -> Result<(), DeviceError> {
        self.do_realize()
    }
}
