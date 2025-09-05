/// Device Types.
///
/// On top of the queues, config space and feature negotiation facilities built
/// into virtio, several devices are defined. The following device IDs are used
/// to identify different types of virtio devices. Some device IDs are reserved
/// for devices which are not currently defined in this standard. Discovering
/// what devices are available and their type is bus-dependent.
#[repr(u32)]
#[allow(dead_code)]
#[derive(Debug, num_enum::TryFromPrimitive)]
pub enum DeviceType {
    Reserved = 0,
    NetworkCard = 1,
    Block = 2,
    Console = 3,
    EntropySource = 4,
    MemoryBallonningTranditional = 5,
    IoMemory = 6,
    RpMsg = 7,
    ScsiHost = 8,
    Transport9P = 9,
    Mac802WLan = 10,
    RprocSerial = 11,
    VirtioCaif = 12,
    MemoryBalloon = 13,
    Gpu = 16,
    TimerClock = 17,
    Input = 18,
    Socket = 19,
    Crypto = 20,
    SignalDistributionModule = 21,
    Pstore = 22,
    IoMmu = 23,
    Memory = 24,
}

bitflags::bitflags! {
    /// VirtIo Device Status.
    #[repr(transparent)]
    pub struct Status: u8 {
        /// Indicates that the guest OS has found the device and recognized it
        /// as a valid virtio device.
        const ACKNOWLEDGE = 1 << 0;
        /// Indicates that the guest OS knows how to drive the device.
        ///
        /// Note: There could be a significant (or infinite) delay before
        /// setting this bit. For example, under Linux, drivers can be loadable
        /// modules.
        const DRIVER = 1 << 1;
        /// Indicates that the driver is set up and ready to drive the device.
        const DRIVER_OK = 1 << 2;
        /// Indicates that the driver has acknowledged all the features it
        /// understands, and feature negotiation is complete.
        const FEATURES_OK = 1 << 3;
        /// Indicates that the device has experienced an error from which it
        /// can't recover.
        const DEVICE_NEEDS_RESET = 1 << 6;
        /// Indicates that something went wrong in the guest, and it has given
        /// up on the device. This could be an internal error, or the driver
        /// didn't like the device for some reason, or even a fatal error during
        /// device operation.
        const FAILED = 1 << 7;
    }
}

bitflags::bitflags! {
    pub struct VirtIoFeaturesCommon: u64 {
        // 6.3 Legacy
        /// If this feature has been negotiated by driver, the device MUST issue a used buffer notification if the device runs out of available descriptors on a virtqueue, even though notifications are suppressed using the VIRTQ_AVAIL_F_NO_INTERRUPT flag or the used_event field. Note: An example of a driver using this feature is the legacy networking driver: it doesn’t need to know every time a packet is transmitted, but it does need to free the transmitted packets a finite time after they are transmitted. It can avoid using a timer if the device notifies it when all the packets are transmitted.
        const NOTIFY_ON_EMPTY = 1 << 24;
        /// This feature indicates that the device accepts arbitrary descriptor layouts, as described in Section 2.6.4.3 Legacy Interface: Message Framing.
        const ANY_LAYOUT = 1 << 27;

        // 6. Reserved Feature Bits
        /// Negotiating this feature indicates that the driver can use descriptors with the VIRTQ_DESC_F_INDIRECT flag set, as described in 2.6.5.3 Indirect Descriptors and 2.7.7 Indirect Flag: Scatter-Gather Support.
        const RING_INDIRECT_DESC = 1 << 28;
        /// This feature enables the used_event and the avail_event fields as described in 2.6.7, 2.6.8 and 2.7.10.
        const RING_EVENT_IDX = 1 << 29;
        /// This indicates compliance with this specification, giving a simple way to detect legacy devices or drivers.
        const VERSION_1 = 1 << 32;
        /// This feature indicates that the device can be used on a platform where device access to data in memory is limited and/or translated. E.g. this is the case if the device can be located behind an IOMMU that translates bus addresses from the device into physical addresses in memory, if the device can be limited to only access certain memory addresses or if special commands such as a cache flush can be needed to synchronise data in memory with the device. Whether accesses are actually limited or translated is described by platform-specific means. If this feature bit is set to 0, then the device has same access to memory addresses supplied to it as the driver has. In particular, the device will always use physical addresses matching addresses used by the driver (typically meaning physical addresses used by the CPU) and not translated further, and can access any address supplied to it by the driver. When clear, this overrides any platform-specific description of whether device access is limited or translated in any way, e.g. whether an IOMMU may be present.
        const ACCESS_PLATFORM = 1 << 33;
        /// This feature indicates support for the packed virtqueue layout as described in 2.7 Packed Virtqueues.
        const RING_PACKED = 1 << 34;
        /// This feature indicates that all buffers are used by the device in the same order in which they have been made available.
        const IN_ORDER = 1 << 35;
        /// This feature indicates that memory accesses by the driver and the device are ordered in a way described by the platform. If this feature bit is negotiated, the ordering in effect for any memory accesses by the driver that need to be ordered in a specific way with respect to accesses by the device is the one suitable for devices described by the platform. This implies that the driver needs to use memory barriers suitable for devices described by the platform; e.g. for the PCI transport in the case of hardware PCI devices.
        ///  this feature bit is not negotiated, then the device and driver are assumed to be implemented in software, that is they can be assumed to run on identical CPUs in an SMP configuration. Thus a weaker form of memory barriers is sufficient to yield better performance.
        const ORDER_PLATFORM = 1 << 36;
        /// This feature indicates that the device supports Single Root I/O Virtualization. Currently only PCI devices support this feature.
        const SR_IOV = 1 << 37;
        /// This feature indicates that the driver passes extra data (besides identifying the virtqueue) in its device notifications. See 2.7.23 Driver notifications.
        const NOTRIFICATION_DATA = 1 << 38;
    }
}

bitflags::bitflags! {
    pub struct IsrCfg: u32 {
        const QUEUE_INTERRUPT = 1 << 0;
        const DEVICE_CONFIGURATION_INTERRUPT = 1 << 1;
    }
}
