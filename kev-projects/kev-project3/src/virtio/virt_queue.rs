//! Virtqueue implementation
use super::VirtIoMmioHeader;
use alloc::{boxed::Box, vec::Vec};
use core::{
    fmt::Debug,
    ptr::{read_volatile, write_volatile},
};
use keos::{
    addressing::{Kva, Pa},
    KernelError,
};

/// Command for the virtqueue.
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum VirtQueueEntryCmd {
    /// Read
    Read = 0,
    /// Write
    Write = 1,
}

/// An entry for the virtqueue.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct VirtQueueEntry {
    pub addr: Pa,
    pub size: usize,
    pub sector: usize,
    pub cmd: VirtQueueEntryCmd,
}

/// A container for holding virtqueue.
#[repr(C)]
pub struct VirtQueue<T>
where
    T: core::ops::Deref<Target = [VirtQueueEntry]>,
{
    entries: T,
}

impl<T> core::ops::Index<usize> for VirtQueue<T>
where
    T: core::ops::Deref<Target = [VirtQueueEntry]>,
{
    type Output = VirtQueueEntry;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.entries.len());
        unsafe {
            ((self.entries.as_ptr() as *const _ as usize
                + core::mem::size_of::<VirtQueueEntry>() * index)
                as *const VirtQueueEntry)
                .as_ref()
                .unwrap()
        }
    }
}
impl<T> core::ops::IndexMut<usize> for VirtQueue<T>
where
    T: core::ops::Deref<Target = [VirtQueueEntry]> + core::ops::DerefMut,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.entries.len());
        unsafe {
            ((self.entries.as_mut_ptr() as *mut _ as usize
                + core::mem::size_of::<VirtQueueEntry>() * index)
                as *mut VirtQueueEntry)
                .as_mut()
                .unwrap()
        }
    }
}

impl VirtQueue<Box<[VirtQueueEntry]>> {
    /// Create a new virtqueue.
    pub fn new(size: usize) -> Self {
        let entries = (0..size)
            .map(|_| VirtQueueEntry {
                addr: Pa::ZERO,
                size: 0,
                sector: 0,
                cmd: VirtQueueEntryCmd::Read,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        VirtQueue { entries }
    }
    /// Get a virtual address of the virtqueue.
    pub fn virt_queue_ptr(&self) -> usize {
        self.entries.as_ptr() as *const _ as usize
    }
}
impl VirtQueue<&'static [VirtQueueEntry]> {
    /// Get a virtqueue from Kva.
    ///
    /// # Safety
    /// `Kva` must be valid address that points to the virtqueue.
    pub unsafe fn new_from_raw_ptr(size: usize, queue_kva: Kva) -> Self {
        let entries = unsafe {
            core::slice::from_raw_parts(queue_kva.into_usize() as *mut VirtQueueEntry, size)
        };

        VirtQueue { entries }
    }
}

impl<T> VirtQueue<T>
where
    T: core::ops::Deref<Target = [VirtQueueEntry]>,
{
    /// Get a fetcher object of the virtqueue.
    pub fn fetcher<'a>(&'a mut self, mmio: &'a mut VirtIoMmioHeader) -> VirtQueueFetcher<'a, T> {
        let head = unsafe { read_volatile(&mmio.queue_head as *const u32) as usize };
        let tail = unsafe { read_volatile(&mmio.queue_tail as *const u32) as usize };
        VirtQueueFetcher {
            inner: self,
            mmio,
            head,
            tail,
        }
    }
}

/// Fetcher object for the virtqueue.
pub struct VirtQueueFetcher<'a, T>
where
    T: core::ops::Deref<Target = [VirtQueueEntry]>,
{
    inner: &'a mut VirtQueue<T>,
    mmio: &'a mut VirtIoMmioHeader,
    head: usize,
    tail: usize,
}

impl<'a, T> VirtQueueFetcher<'a, T>
where
    T: core::ops::Deref<Target = [VirtQueueEntry]>,
{
    fn charge(&self) -> usize {
        self.head - self.tail
    }

    fn size(&self) -> usize {
        self.inner.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    fn is_full(&self) -> bool {
        self.size() == self.charge()
    }
}

impl<'a> VirtQueueFetcher<'a, &'static [VirtQueueEntry]> {
    /// Pop a single entry to the virtqueue.
    pub fn pop_back(&mut self) -> Option<VirtQueueEntry> {
        if !self.is_empty() {
            let size = self.size();
            let r = self.inner.entries[self.tail];
            self.tail = (self.tail + 1) % size;
            Some(r)
        } else {
            None
        }
    }
    /// Acknowledge the consumed request.
    pub fn ack(self) -> Result<(), KernelError> {
        // The sequence of the update in this function
        // is really important. Do not change the order.
        unsafe {
            if read_volatile(&self.mmio.queue_tail) != self.tail as u32 {
                write_volatile(&mut self.mmio.queue_tail, self.tail as u32);
            }
            // This check is required to verify the change we made into mmio area.
            if read_volatile(&self.mmio.queue_head) == self.head as u32
                && read_volatile(&self.mmio.queue_tail) == self.tail as u32
            {
                if read_volatile(&self.mmio.status) != super::VirtIoStatus::READY as u32 {
                    return Err(KernelError::IOError);
                }
                Ok(())
            } else {
                Err(KernelError::IOError)
            }
        }
    }
}

impl<'a> VirtQueueFetcher<'a, Box<[VirtQueueEntry]>> {
    /// Push a single entry to the virtqueue.
    ///
    /// This does not ring the doorbell.
    pub fn push_front(&mut self, value: VirtQueueEntry) -> Result<(), VirtQueueEntry> {
        if !self.is_full() {
            let size = self.size();
            self.inner.entries[self.head] = value;
            self.head = (self.head + 1) % size;
            Ok(())
        } else {
            Err(value)
        }
    }

    /// Kick the doorbell to request commands to the VMM.
    pub fn kick(mut self) -> Result<(), KernelError> {
        // The sequence of the update in this function
        // is really important. Do not change the order.
        unsafe {
            if read_volatile(&self.mmio.queue_head) != self.head as u32 {
                write_volatile(&mut self.mmio.queue_head, self.head as u32);
                self.tail = read_volatile(&self.mmio.queue_tail) as usize;
            }
            // This check is required to verify the change we made into mmio area.
            if read_volatile(&self.mmio.queue_head) == self.head as u32
                && read_volatile(&self.mmio.queue_tail) == self.tail as u32
            {
                if read_volatile(&self.mmio.status) != super::VirtIoStatus::READY as u32 {
                    return Err(KernelError::IOError);
                }
                Ok(())
            } else {
                Err(KernelError::IOError)
            }
        }
    }
}

impl<'a, T> Debug for VirtQueueFetcher<'a, T>
where
    T: core::ops::Deref<Target = [VirtQueueEntry]>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VirtQueue")
            .field("size", &self.inner.entries.len())
            .field("head", &self.head)
            .field("tail", &self.tail)
            .finish()
    }
}
