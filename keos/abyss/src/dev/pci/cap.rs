use super::{PciAccessor, PciHeader};

/// Device's capability.
pub struct Capability<'a, const V: usize> {
    base: u8,
    pci_header: &'a PciHeader<V>,
}

impl<const V: usize> Capability<'_, V> {
    /// Get vendor id of the capability.
    #[inline]
    pub fn vendor(&self) -> u8 {
        self.pci_header.accessor(self.base).read_u8()
    }

    /// Get accessor to read/write the vendor specific capabilities.
    #[inline]
    pub fn offset(&self, offset: u8) -> PciAccessor {
        PciAccessor::new(
            self.pci_header.pci_device.bus,
            self.pci_header.pci_device.device,
            self.pci_header.function,
            self.base + offset,
        )
    }
}

#[doc(hidden)]
pub struct CapabilityIterator<'a, const V: usize> {
    pub(crate) next: u8,
    pub(crate) pci_header: &'a PciHeader<V>,
}

impl<'a, const V: usize> core::iter::Iterator for CapabilityIterator<'a, V> {
    type Item = Capability<'a, V>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next != 0 {
            let cur = self.next;
            self.next = self.pci_header.accessor(cur + 1).read_u8();

            Some(Capability {
                base: cur,
                pci_header: self.pci_header,
            })
        } else {
            None
        }
    }
}

bitflags::bitflags! {
    pub struct MsixMessageControl: u16 {
        const ENABLED = 1 << 15;
        const FUNCTION_MASK = 1 << 14;
    }
}

/// Msix message control accessor
pub struct MessageControl {
    accessor: PciAccessor,
}

impl MessageControl {
    /// Set underlying value.
    #[inline]
    pub fn set(&self, ctrl: MsixMessageControl) {
        let cap = unsafe {
            MsixMessageControl::from_bits_unchecked(self.accessor.read_u16())
                & !(MsixMessageControl::ENABLED | MsixMessageControl::FUNCTION_MASK)
                | ctrl
        };
        self.accessor.write_u16(cap.bits())
    }

    /// Get underlying value.
    #[inline]
    pub fn get(&self) -> MsixMessageControl {
        MsixMessageControl::from_bits_truncate(self.accessor.read_u16())
    }
}
