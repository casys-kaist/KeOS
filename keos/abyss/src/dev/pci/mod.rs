//! Pci discovery and operations.

mod bar;
mod cap;
mod header;
pub mod virtio;
mod x86_config;

pub use bar::{Bar, IoSpace, MemorySpace};
pub use cap::{Capability, CapabilityIterator, MessageControl};
pub use header::*;
use x86_config::X86Config;

/// Representation of PciDevice.
#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    bus: u8,
    device: u8,
}

impl PciDevice {
    /// Get iterator to enumerate the available functions of the device.
    pub fn functions(&self) -> PciDeviceFunctionIterator {
        if get_header_type(self.bus, self.device, 0) & 0x80 == 0x80 {
            PciDeviceFunctionIterator {
                pci_device: *self,
                function: 0..8,
            }
        } else {
            PciDeviceFunctionIterator {
                pci_device: *self,
                function: 0..1,
            }
        }
    }
}

#[doc(hidden)]
pub struct PciDeviceFunctionIterator {
    pci_device: PciDevice,
    function: core::ops::Range<u8>,
}

impl core::iter::Iterator for PciDeviceFunctionIterator {
    type Item = PciDeviceHeader;

    fn next(&mut self) -> Option<Self::Item> {
        for function in &mut self.function {
            if get_vendor_id(self.pci_device.bus, self.pci_device.device, function) != 0xffff {
                match get_header_type(self.pci_device.bus, self.pci_device.device, function) & 0x7f
                {
                    0 => {
                        return Some(PciDeviceHeader::Type0(PciHeader {
                            pci_device: self.pci_device,
                            function,
                        }));
                    }
                    1 => {
                        return Some(PciDeviceHeader::Type1(PciHeader {
                            pci_device: self.pci_device,
                            function,
                        }));
                    }
                    2 => {
                        return Some(PciDeviceHeader::Type2(PciHeader {
                            pci_device: self.pci_device,
                            function,
                        }));
                    }
                    ty => panic!("Unknonw ty: {:?}", ty),
                }
            }
        }
        None
    }
}

/// Access helper for Pci Device.
#[derive(Debug)]
pub struct PciAccessor {
    addr: usize,
    max_access: u32,
}

impl PciAccessor {
    #[inline]
    pub(crate) fn new(bus: u8, slot: u8, func: u8, offset: u8) -> Self {
        Self {
            addr: X86Config.make_address(bus, slot, func, offset),
            max_access: if offset < 255 - 2 {
                4
            } else if offset < 255 {
                2
            } else {
                1
            },
        }
    }

    /// Write u8 to pci address.
    #[inline]
    pub fn write_u8(&self, v: u8) {
        X86Config.write_u8(self.addr, v)
    }

    /// Write u16 to pci address.
    #[inline]
    pub fn write_u16(&self, v: u16) {
        if self.max_access < 2 {
            panic!("Invalid write: {:?}", self.addr & 0xff)
        }
        X86Config.write_u16(self.addr, v)
    }

    /// Write u32 to pci address.
    #[inline]
    pub fn write_u32(&self, v: u32) {
        if self.max_access < 4 {
            panic!("Invalid write: {:?}", self.addr & 0xff)
        }
        X86Config.write_u32(self.addr, v)
    }

    /// read u8 from pci address.
    #[inline]
    pub fn read_u8(&self) -> u8 {
        X86Config.read_u8(self.addr)
    }

    /// read u16 from pci address.
    #[inline]
    pub fn read_u16(&self) -> u16 {
        if self.max_access < 2 {
            panic!("Invalid write: {:?}", self.addr & 0xff)
        }
        X86Config.read_u16(self.addr)
    }

    /// read u32 from pci address.
    #[inline]
    pub fn read_u32(&self) -> u32 {
        if self.max_access < 4 {
            panic!("Invalid write: {:?}", self.addr & 0xff)
        }
        X86Config.read_u32(self.addr)
    }
}

/// A Enumeration of multiple types of Pci Device Headers.
#[derive(Debug, Clone, Copy)]
pub enum PciDeviceHeader {
    /// Type zero.
    Type0(PciHeader<0>),
    /// Type one.
    Type1(PciHeader<1>),
    /// Type two.
    Type2(PciHeader<2>),
}

/// Pci device's `Device Id` and `Vendor Id`.
#[derive(Debug)]
pub struct DeviceVendor {
    /// Device id.
    pub dev_id: u16,
    /// Vendor id.
    pub vendor_id: u16,
}

impl PciDeviceHeader {
    #[inline]
    fn bus_device_function(&self) -> (u8, u8, u8) {
        match self {
            Self::Type0(PciHeader {
                pci_device: PciDevice { bus, device },
                function,
            })
            | Self::Type1(PciHeader {
                pci_device: PciDevice { bus, device },
                function,
            })
            | Self::Type2(PciHeader {
                pci_device: PciDevice { bus, device },
                function,
            }) => (*bus, *device, *function),
        }
    }

    /// Get device and vendor information of the device.
    pub fn device_vendor(&self) -> DeviceVendor {
        let (bus, device, function) = self.bus_device_function();

        let of0 = PciAccessor::new(bus, device, function, 0).read_u32();
        DeviceVendor {
            dev_id: (of0 >> 16) as u16,
            vendor_id: of0 as u16,
        }
    }

    /// Get class of the device.
    pub fn class(&self) -> PciDeviceClass {
        let (bus, device, function) = self.bus_device_function();
        let (hi_byte, lo_byte) = {
            let of8 = (PciAccessor::new(bus, device, function, 0x8).read_u32() >> 16) as u16;
            ((of8 >> 8) as u8, of8 as u8)
        };
        match (hi_byte, lo_byte) {
            (1, 0) => PciDeviceClass::ScsiBusController,
            (1, 1) => PciDeviceClass::IdeController,
            (2, 0) => PciDeviceClass::EthernetController,
            (3, 0) => PciDeviceClass::VgaCompatController,
            (6, 0) => PciDeviceClass::HostBridge,
            (6, 1) => PciDeviceClass::IsaBridge,
            (6, 4) => PciDeviceClass::PciToPciBridge,
            (6, 0x80) => PciDeviceClass::OtherBridge,
            (base_class, sub_class) => PciDeviceClass::UnknownClass(base_class, sub_class),
        }
    }

    pub fn get_secondary_bus(&self) -> Option<u8> {
        match self {
            Self::Type1(PciHeader {
                pci_device: PciDevice { bus, device, .. },
                function,
            }) => Some((PciAccessor::new(*bus, *device, *function, 0x18).read_u16() >> 8) as u8),
            _ => None,
        }
    }
}

/// Enumeration of pci device class
///
/// See <https://wiki.osdev.org/Pci#Class_Codes> for more class.
#[derive(Debug)]
pub enum PciDeviceClass {
    /// Scsi Bus controller
    // 1 0
    ScsiBusController,
    /// Ide controller
    // 1 1
    IdeController,
    /// Ethernet controller
    // 2 0
    EthernetController,
    /// VGA compatable controller
    // 3 0
    VgaCompatController,
    /// Host bridge
    // 6 0
    HostBridge,
    /// Isa bridge
    // 6 1
    IsaBridge,
    /// Pci to Pci bridge
    // 6 4
    PciToPciBridge,
    /// Other bridge
    // 6 128
    OtherBridge,
    /// Unidentified class
    UnknownClass(u8, u8),
}

#[inline]
fn get_header_type(bus: u8, device: u8, function: u8) -> u8 {
    (PciAccessor::new(bus, device, function, 0xC).read_u32() >> 16) as u8
}

#[inline]
fn get_vendor_id(bus: u8, device: u8, function: u8) -> u16 {
    PciAccessor::new(bus, device, function, 0x0).read_u16()
}

/// Iterator for Pci Devices.
pub struct PciIterator {
    bus: u16,
    device: u8,
    max_bus: u16,
}

impl core::iter::Iterator for PciIterator {
    type Item = PciDevice;

    fn next(&mut self) -> Option<Self::Item> {
        while self.bus <= self.max_bus {
            if self.device < 32 {
                self.device += 1;
                if get_vendor_id(self.bus as u8, self.device - 1, 0) != 0xffff {
                    return Some(PciDevice {
                        bus: self.bus as u8,
                        device: self.device - 1,
                    });
                }
            } else {
                self.device = 0;
                self.bus += 1;
            }
        }
        None
    }
}

/// Scan a single pci bus.
#[inline]
pub fn scan_bus(bus: u8) -> PciIterator {
    PciIterator {
        bus: bus as u16,
        device: 0,
        max_bus: bus as u16,
    }
}

/// Scan whole pci buses.
#[inline]
pub fn scan() -> PciIterator {
    PciIterator {
        bus: 0,
        device: 0,
        max_bus: 255,
    }
}

/// Initialize pci devices.
pub unsafe fn init() {
    unsafe {
        // Scan pci bus
        for dev in scan().flat_map(|dev| dev.functions()) {
            match dev.device_vendor() {
                DeviceVendor {
                    dev_id: 0x1001,
                    vendor_id: 0x1af4,
                } => {
                    let dev = virtio::block::VirtIoBlock::from_pci(dev)
                        .expect("Failed to create virtio block device.");
                    for slot in super::BLOCK_DEVS.iter_mut() {
                        if slot.is_none() {
                            *slot = Some(dev);
                            slot.as_ref()
                                .unwrap()
                                .init()
                                .expect("Failed to initialize virtio block device.");
                            break;
                        }
                    }
                }
                _dev => (),
            }
        }
    }
}
