use super::bar::{Bar, IoSpace, MemorySpace};
use super::cap::CapabilityIterator;
use super::{PciAccessor, PciDevice};
use crate::addressing::Pa;

bitflags::bitflags! {
    /// Pci device status.
    pub struct Status: u16 {
        /// This bit will be set to 1 whenever the device detects a parity error, even if parity error handling is disabled.
        const DETECTED_PARITY_ERROR = 1 << 15;
        /// This bit will be set to 1 whenever the device asserts SERR#.
        const SIGNALLED_SYSTEM_ERROR = 1 << 14;
        /// This bit will be set to 1, by a master device, whenever its transaction (except for Special Cycle transactions) is terminated with Master-Abort.
        const RECEIVED_MASTER_ABORT = 1 << 13;
        /// This bit will be set to 1, by a master device, whenever its transaction is terminated with Target-Abort.
        const RECEIVED_TARGET_ABORT = 1 << 12;
        /// This bit will be set to 1 whenever a target device terminates a transaction with Target-Abort.
        const SIGNALLED_TARGET_ABORT = 1 << 11;
        /// Read only bits that represent the slowest time that a device will assert DEVSEL# for any bus command except Configuration Space read and writes. Where a value of 0x00 represents fast timing, a value of 0x01 represents medium timing, and a value of 0x02 represents slow timing.
        const DEVSEL_TIMING = 1 << 10;
        /// This bit is only set when the following conditions are met. The bus agent asserted PERR# on a read or observed an assertion of PERR# on a write, the agent setting the bit acted as the bus master for the operation in which the error occurred, and bit 6 of the Command register (Parity Error Response bit) is set to 1.
        const MASTER_DATA_PARITY_ERROR = 1 << 9;
        /// If set to 1 the device can accept fast back-to-back transactions that are not from the same agent; otherwise, transactions can only be accepted from the same agent.
        const FAST_BACK_TO_BACK_CAPABLE = 1 << 8;
        /// If set to 1 the device is capable of running at 66 MHz; otherwise, the device runs at 33 MHz.
        const MHZ66_CAPABLE = 1 << 5;
        /// If set to 1 the device implements the pointer for a New Capabilities Linked list at offset 0x34; otherwise, the linked list is not available.
        const CAPABILITIES_LIST = 1 << 4;
        /// Represents the state of the device's INTx# signal. If set to 1 and bit 10 of the Command register (Interrupt Disable bit) is set to 0 the signal will be asserted; otherwise, the signal will be ignored
        const INTERRUPT_STATUS = 1 << 3;
    }
}

// register    offset  bits 31-24  bits 23-16  bits 15-8   bits 7-0
// 00  00  Device ID   Vendor ID
// 01  04  Status  Command
// 02  08  Class code  Subclass    Prog IF Revision ID
// 03  0C  BIST    Header type Latency Timer   Cache Line Size
// 04  10  Base address #0 (BAR0)
// 05  14  Base address #1 (BAR1)
// 06  18  Base address #2 (BAR2)
// 07  1C  Base address #3 (BAR3)
// 08  20  Base address #4 (BAR4)
// 09  24  Base address #5 (BAR5)
// 0A  28  Cardbus CIS Pointer
// 0B  2C  Subsystem ID    Subsystem Vendor ID
// 0C  30  Expansion ROM base address
// 0D  34  Reserved    Capabilities Pointer
// 0E  38  Reserved
// 0F  3C  Max latency Min Grant   Interrupt PIN   Interrupt Line
/// Generic implementation of PciHeaderType.
#[derive(Debug, Clone, Copy)]
pub struct PciHeader<const V: usize> {
    pub(crate) pci_device: PciDevice,
    pub(crate) function: u8,
}

impl<const V: usize> PciHeader<V> {
    #[inline(always)]
    pub fn accessor(&self, off: u8) -> PciAccessor {
        PciAccessor::new(
            self.pci_device.bus,
            self.pci_device.device,
            self.function,
            off,
        )
    }
}

impl PciHeader<0> {
    /// Get status of the device.
    #[inline]
    pub fn status(&self) -> Status {
        Status::from_bits_truncate((self.accessor(0x4).read_u32() >> 16) as u16)
    }

    /// Get iterator for enumerating the capabilties of device.
    #[inline]
    pub fn capabilities(&self) -> CapabilityIterator<'_, 0> {
        CapabilityIterator {
            next: if self.status().contains(Status::CAPABILITIES_LIST) {
                self.accessor(0x34).read_u8()
            } else {
                0
            },
            pci_header: self,
        }
    }

    /// Get BAR of the device.
    #[inline]
    pub fn bar(&self, index: u8) -> Option<Bar> {
        if index > 5 {
            return None;
        }
        let bar_accessor = self.accessor(0x10 + 4 * index);
        let bar = bar_accessor.read_u32();

        // Resolve length.
        bar_accessor.write_u32(u32::MAX);
        let bar_length = bar_accessor.read_u32();
        bar_accessor.write_u32(bar);

        if bar & 1 == 1 {
            Some(Bar::IoSpace(IoSpace {
                base: bar & !3,
                length: !(bar_length & !0b11) + 1,
            }))
        } else {
            let (prefetchable, ty) = (bar & 8 == 8, (bar >> 1) & 3);
            let addr = match ty {
                0 => (bar & !0xf) as usize,
                2 if index <= 4 => {
                    (((bar & !0xf) as u64)
                        | ((self.accessor(0x10 + 4 * index + 4).read_u32() as u64) << 32))
                        as usize
                }
                // 1 => reserved.
                _ => return None,
            };
            Pa::new(addr).map(|base| {
                Bar::MemorySpace(MemorySpace {
                    base,
                    length: (!(bar_length & !0b1111) + 1) as usize,
                    _prefetchable: prefetchable,
                })
            })
        }
    }
}
