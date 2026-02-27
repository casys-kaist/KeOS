//! Simple VirtIO Block
//!
//! ## Background
//! ### Simple VirtIO Block device specification
//! ### 1. Introduction
//! This describes the specifications of the "Simple VirtIO Block" device.
//! The purpose of this documentations is to provide the basic and concrete
//! informations for building svirtb-device and svirtb-driver.
//!
//! #### 1.1 Terminology
//! The keywords "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
//! "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to
//! be interpreted as described in [RFC2119](http://www.ietf.org/rfc/rfc2119.txt).
//!
//! #### 1.2 Structure Specification
//! Device and In-memory structure layouts are documented using the C struct
//! syntax. All structure MUST not have any additional padding.
//!
//! For the integer data types, the following conventions are used:
//! * u8, u16, u32, u64: An unsigned integer of the specified length in bits.
//! * le8, le16, le32, le64: An unsigned integer of the specified length in
//!   bits, in little-endian byte order.
//!
//! ### 2. Basic Facilities of a Simple VirtIO Block
//! A simple virtIO block is discovered and identified by a mmio struct
//! [`VirtIoMmioHeader`]. Mmio field (header) consists of the following parts:
//! ```C
//! struct VirtIoMmioHeader {
//!     le32 status;
//!     le32 queue_size;
//!     le32 queue_addr_hi;
//!     le32 queue_addr_lo;
//!     le32 queue_head;
//!     le32 queue_tail;
//! }
//! ```
//! * status: Status of the device
//! * queue_size: Size of the virtqueue
//! * queue_addr_hi: Upper 32bit of the virtqueue physical address
//! * queue_addr_lo: Lower 32bit of the virtqueue physical address
//! * queue_head: Head of the ring buffer. Driver update the tail of the queue.
//!   Device must not update the field.
//! * queue_tail: Tail of the ring buffer. Device update the tail of the queue.
//!   Driver must not update the field.
//!
//! #### 2.1 Device status field
//! During device initialization by a driver, the driver follows the sequence of
//! steps specified in [`3`](#3-device-initialization). The device status field
//! provides a simple low-level indication of the completed steps of this
//! sequence. It's most useful to imagine it hooked up to traffic lights on the
//! console indicating the status of each device. The following status code are
//! defined (listed below in the order in which they would be typically set):
//! ```C
//! le32 MAGIC = 0x74726976
//! le32 DRIVER_OK = 0
//! le32 READY = 1
//! le32 RESET = 2
//! ```
//! * MAGIC : Indicates that the region of the mmio is used for svirtb. Device
//!   sets the fields when initialization.
//! * DRIVER_OK : Indicates that the driver is ready to initiate the svirtb.
//! * READY : Indicates that the device is ready to be used by the svirtb device
//!   driver. From this points, svirtb device is activated.
//! * RESET : Reset the svirtb connection and start over.
//!
//! #### 2.1.1 Driver Requirements: Device Status Field
//! The driver MUST update device status, setting bits to indicate the steps of
//! the driver initialization in [`3`](#3-device-initialization). The driver
//! MUST NOT set a device status to MAGIC value. The driver MUST fill the
//! necessary fields of the ring buffer configuration space
//! (size, queue_addr_low and queue_addr_high) after driver sets the DRIVER_OK
//! status bit.
//!
//! #### 2.1.2 Device Requirements: Device Status Field
//! The device MUST initialize device status to 0x74726976 upon start and reset.
//!
//! The driver MUST enable READY after it validate the queue address and size
//! negotiated with the virtio driver.
//! The device SHOULD set RESET when it enters an error state that a reset is
//! needed.
//!
//! #### 2.2 Ring buffer
//! The mechanisms for data transport on svirtb is ring buffer.
//! Driver makes requests available to device by enqueue an entry or multiple
//! entries to the ring buffer and write the index to the doorbell
//! (ring_buffer_head). Device executes the requests and, when complete, update
//! the lastly consume index to the ring_buffer_tail.
//!
//! #### 2.2.1 Ring buffer entry
//! Each entry of the ring buffer is consists of the four parts.
//! ```C
//! struct VirtQueueEntry {
//!     le64 addr;
//!     le64 size;
//!     le64 sector;
//!     le32 cmd;
//! }
//! ```
//! * addr: Physical address of the buffer
//! * size: size of the buffer
//! * sector: sector of the vritual disk
//! * cmd: indicates the command. 0 is read, and 1 is write.
//!
//! ### 3. Device Initialization
//! The driver MUST follow this sequence to initialize a device:
//! 1. Check the magic exists in status field.
//! 2. Write the DRIVER_OK to the status field.
//! 3. Check whether status field is still DRIVER_OK.
//! 4. Perform ring buffer configuration; reading and writing device's ring
//!    buffer configuration space.
//! 5. Set the status field to READY.
//! 6. Check whether status field is READY. At this point the device is "live".
//!
//! The driver MUST NOT send any buffer available notifications to the device
//! before setting READY.
//!
//! ### 4. virtio over mmio
//! For simplicity of the implementation, the mmio region of the simple virtIO
//! block always located on the 0xcafe0000.
//!
//! The layout of the mmio area is follow:
//! ```C
//! struct svirtb {
//!     le32 status;
//!     le32 queue_size;
//!     le32 queue_addr_hi;
//!     le32 queue_addr_lo;
//!     le32 queue_head;
//!     le32 queue_tail;
//! }
//! ```
//! * status: Device status bits. Reading from this register returns the current
//!   device status flags. Initialized with magic by device - 0x74726976 (a
//!   Little Endian equivalent of the 'virt' string).
//! * queue_size: Writing to this register notifies the device what size of the
//!   queue the driver will use (select the length of the queue).
//! * queue_addr_hi: high part of ring buffer physical address (32-63 bits)
//! * queue_addr_lo: low part of ring buffer physical address (0-31 bits)
//! * queue_head: head index of the ring buffer
//! * queue_tail: tail index of the ring buffer
//!
//! ## Tasks
//! In this project, you are required to implement the device part (backend
//! driver) of Simple Virtio Block Device. You can get [`VirtQueue`] by calling
//! [`VirtQueue::new_from_raw_ptr`]. After that, you can access
//! [`VirtQueueEntry`] through an struct called [`VirtQueueFetcher`].
//! You can utilize [`VirtQueueFetcher`] to implement this project.
//!
//! [`VirtQueue`]: crate::virtio::virt_queue::VirtQueue::new_from_raw_ptr
//! [`VirtQueueEntry`]: crate::virtio::virt_queue::VirtQueueEntry
//! [`VirtQueueFetcher`]: crate::virtio::virt_queue::VirtQueueFetcher
use crate::virtio::{
    virt_queue::{VirtQueue, VirtQueueEntry, VirtQueueEntryCmd},
    VirtIoMmioHeader, VirtIoStatus,
};
use alloc::{ sync::Arc};
use core::mem::size_of;
use keos::{
    fs::{FileSystem, RegularFile},
    sync::SpinLock,
};
use kev::{
    vcpu::{GenericVCpuState, VmexitResult},
    vm::Gpa,
    Probe, VmError,
};
use kev_project2::{
    ept::EptMappingError,
    keos_vm::pager::KernelVmPager,
    vmexit::mmio::{self, MmioInfo, MmioRegion},
};

pub struct SimpleVirtioBlockDevInner {
    _status: VirtIoStatus,
    virt_queue: Option<VirtQueue<&'static [VirtQueueEntry]>>,
    file_system: RegularFile,
}

pub struct SimpleVirtIoBlockDev {
    inner: Arc<SpinLock<SimpleVirtioBlockDevInner>>,
}

impl Default for SimpleVirtIoBlockDev {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleVirtIoBlockDev {
    pub fn new() -> Self {
        let this = SimpleVirtioBlockDevInner {
            _status: VirtIoStatus::MAGIC,
            virt_queue: None,
            file_system: FileSystem::root()
                .open("disk_file")
                .unwrap()
                .into_regular_file()
                .unwrap(),
        };
        Self {
            inner: Arc::new(SpinLock::new(this)),
        }
    }

    pub fn attach(
        &self,
        pager: &mut KernelVmPager,
        mmio_ctl: &mut mmio::Controller,
    ) -> Result<(), EptMappingError> {
        todo!()
    }
}

impl mmio::MmioHandler for SimpleVirtIoBlockDev {
    fn region(&self) -> MmioRegion {
        MmioRegion {
            start: Gpa::new(0xcafe0000).unwrap(),
            end: Gpa::new(0xcafe0000 + size_of::<VirtIoMmioHeader>()).unwrap(),
        }
    }

    fn handle(
        &mut self,
        p: &dyn Probe,
        info: MmioInfo,
        generic_vcpu_state: &mut GenericVCpuState,
    ) -> Result<VmexitResult, VmError> {
        if let mmio::Direction::Write32 { dst, src } = info.direction {
            todo!()
        }
        Ok(VmexitResult::Ok)
    }
}
