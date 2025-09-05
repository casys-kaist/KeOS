use crate::addressing::{Kva, Pa};
use crate::dev::mmio::MmioAccessor;
use alloc::boxed::Box;
use core::sync::atomic::{Ordering, fence};

bitflags::bitflags! {
    pub struct VirtqDescFlags: u16 {
        /// The buffer is continuing via the next field
        const NEXT = 1 << 0;
        /// The buffer is device write-only (otherwise device read-only).
        const WRITE = 1 << 1;
        /// The buffer contains a list of buffer descriptors.
        const INDIRECT = 1 << 2;
    }
}

/// The Virtqueue Descriptor Table.
///
/// The descriptor table refers to the buffers the driver is using for the
/// device. addr is a physical address, and the buffers can be chained via next.
/// Each descriptor describes a buffer which is read-only for the device
/// (“device-readable”) or write-only for the device (“device-writable”), but a
/// chain of descriptors can contain both device-readable and device-writable
/// buffers. The actual contents of the memory offered to the device depends on
/// the device type. Most common is to begin the data with a header (containing
/// little-endian fields) for the device to read, and postfix it with a status
/// tailer for the device to write.
#[repr(C, align(16))]
#[derive(Debug)]
pub struct VirtqDesc {
    /// Address (guest-physical)
    pub addr: Pa,
    /// Length.
    pub len: u32,
    /// The flags.
    pub flags: VirtqDescFlags,
    /// Next field if flags & NEXT.
    next: u16,
}

pub struct VirtqDescs {
    inner: [VirtqDesc; 0],
    _pin: core::marker::PhantomPinned,
}

pub struct VirtqDescContainer {
    inner: Box<VirtqDescs>,
    size: usize,
}

impl VirtqDescContainer {
    pub fn new(size: usize) -> Self {
        let inner = unsafe {
            Box::from_raw(alloc::alloc::alloc_zeroed(
                alloc::alloc::Layout::from_size_align(core::mem::size_of::<VirtqDesc>() * size, 16)
                    .unwrap(),
            ) as *mut VirtqDescs)
        };
        let mut output = Self { inner, size };
        // Build the chain
        (0..size).for_each(|i| {
            if i != size {
                output[i].next = (i + 1) as u16;
            } else {
                output[i].next = 0xffff;
            }
        });
        output
    }

    #[inline]
    pub fn inner(&self) -> &VirtqDescs {
        self.inner.as_ref()
    }
}

impl core::ops::Index<usize> for VirtqDescContainer {
    type Output = VirtqDesc;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.size);
        unsafe {
            ((self.inner.inner.as_ptr() as *const _ as usize
                + core::mem::size_of::<VirtqDesc>() * index) as *const VirtqDesc)
                .as_ref()
                .unwrap()
        }
    }
}

impl core::ops::IndexMut<usize> for VirtqDescContainer {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.size);
        unsafe {
            ((self.inner.inner.as_mut_ptr() as *mut _ as usize
                + core::mem::size_of::<VirtqDesc>() * index) as *mut VirtqDesc)
                .as_mut()
                .unwrap()
        }
    }
}

#[repr(C, align(2))]
pub struct VirtqAvail {
    flags: u16,
    idx: u16,
    rings: [u16; 0],
    _pin: core::marker::PhantomPinned,
}

#[allow(dead_code)]
pub struct VirtqAvailContainer {
    inner: Box<VirtqAvail>,
    size: usize,
    has_used_event: bool,
}

impl VirtqAvailContainer {
    pub fn new(size: usize, has_used_event: bool) -> Self {
        let inner = unsafe {
            Box::from_raw(alloc::alloc::alloc_zeroed(
                alloc::alloc::Layout::from_size_align(
                    core::mem::size_of::<u16>() * (2 + size + if has_used_event { 1 } else { 0 }),
                    2,
                )
                .unwrap(),
            ) as *mut VirtqAvail)
        };

        VirtqAvailContainer {
            inner,
            size,
            has_used_event,
        }
    }

    #[inline]
    pub fn inner(&self) -> &VirtqAvail {
        self.inner.as_ref()
    }

    #[inline]
    pub fn submit_chain(&mut self, index: u16) {
        let idx: usize = self.inner.idx as usize;
        unsafe {
            *((self.inner.rings.as_mut_ptr() as usize
                + core::mem::size_of::<u16>() * (idx % self.size)) as *mut u16) = index;
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            core::ptr::write_volatile(&mut self.inner.idx, idx.wrapping_add(1) as u16)
        }
    }
}

#[repr(C)]
pub struct VirtqUsedElem {
    /// Index of start of used descriptor chain.
    id: u32,
    /// Total length of the descriptor chain which was used (written to)
    len: u32,
}

#[repr(C, align(4))]
pub struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; 0],
    // used_event
    _pin: core::marker::PhantomPinned,
}

pub struct VirtqUsedContainer {
    inner: Box<VirtqUsed>,
    size: usize,
}

impl VirtqUsedContainer {
    pub fn new(size: usize) -> Self {
        let inner = unsafe {
            Box::from_raw(alloc::alloc::alloc_zeroed(
                alloc::alloc::Layout::from_size_align(
                    core::mem::size_of::<u16>() * 3 + core::mem::size_of::<VirtqUsedElem>() * size,
                    4,
                )
                .unwrap(),
            ) as *mut VirtqUsed)
        };

        VirtqUsedContainer { inner, size }
    }

    #[inline]
    pub fn inner(&self) -> &VirtqUsed {
        self.inner.as_ref()
    }

    #[inline]
    pub fn idx(&self) -> u16 {
        self.inner.idx
    }
}

impl core::ops::Index<usize> for VirtqUsedContainer {
    type Output = VirtqUsedElem;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index % self.size]
    }
}

impl core::ops::IndexMut<usize> for VirtqUsedContainer {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.inner[index % self.size]
    }
}

impl core::ops::Index<usize> for VirtqUsed {
    type Output = VirtqUsedElem;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            ((self.ring.as_ptr() as *const _ as usize
                + core::mem::size_of::<VirtqUsedElem>() * index)
                as *const VirtqUsedElem)
                .as_ref()
                .unwrap()
        }
    }
}

impl core::ops::IndexMut<usize> for VirtqUsed {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            ((self.ring.as_mut_ptr() as *mut _ as usize
                + core::mem::size_of::<VirtqUsedElem>() * index) as *mut VirtqUsedElem)
                .as_mut()
                .unwrap()
        }
    }
}

pub enum Kick {
    Pci(MmioAccessor<u16, true, true>),
    None,
}

pub struct VirtQueue {
    pub desc: VirtqDescContainer,
    pub avail: VirtqAvailContainer,
    pub used: VirtqUsedContainer,
    size: u16,
    pub id: u16,
    kick: Kick,
}

impl VirtQueue {
    pub(crate) fn empty() -> Self {
        Self {
            desc: VirtqDescContainer::new(0),
            avail: VirtqAvailContainer::new(0, false),
            used: VirtqUsedContainer::new(0),
            size: 0,
            id: 0,
            kick: Kick::None,
        }
    }

    pub(crate) fn new(size: u16, id: u16, has_used_event: bool, kick: Kick) -> Self {
        Self {
            desc: VirtqDescContainer::new(size as usize),
            avail: VirtqAvailContainer::new(size as usize, has_used_event),
            used: VirtqUsedContainer::new(size as usize),
            size,
            id,
            kick,
        }
    }

    #[inline]
    fn kick(&self, idx: u16) {
        match self.kick {
            Kick::Pci(kick_addr) => kick_addr.write(u16::to_le(idx)),
            Kick::None => unreachable!(),
        }
    }

    #[inline]
    pub fn sgl_builder(&mut self) -> VirtqSglBuilder<'_> {
        VirtqSglBuilder {
            virtq: self,
            idx: 0,
        }
    }

    #[inline]
    pub fn size(&self) -> u16 {
        self.size
    }
}

pub struct VirtqSglBuilder<'a> {
    virtq: &'a mut VirtQueue,
    idx: usize,
}

impl VirtqSglBuilder<'_> {
    #[inline]
    pub fn push<T>(&mut self, val: &T)
    where
        T: ?Sized,
    {
        if self.idx != 0 {
            self.virtq.desc[self.idx - 1].flags |= VirtqDescFlags::NEXT;
        }
        // FIXME: handle concurrently.
        let desc = &mut self.virtq.desc[self.idx];
        self.idx += 1;

        desc.addr = Kva::new(val as *const _ as *const () as usize)
            .unwrap()
            .into_pa();
        desc.len = core::mem::size_of_val(val) as u32;
        desc.flags = VirtqDescFlags::empty();
    }

    #[inline]
    pub fn push_mut<T>(&mut self, val: &mut T)
    where
        T: ?Sized,
    {
        if self.idx != 0 {
            self.virtq.desc[self.idx - 1].flags |= VirtqDescFlags::NEXT;
        }
        // FIXME: handle concurrently.
        let desc = &mut self.virtq.desc[self.idx];
        self.idx += 1;

        desc.addr = Kva::new(val as *const _ as *const () as usize)
            .unwrap()
            .into_pa();
        desc.len = core::mem::size_of_val(val) as u32;
        desc.flags = VirtqDescFlags::WRITE;
    }

    // FIXME: genernalize via trait.
    #[inline]
    pub fn finish(self) -> usize {
        fence(Ordering::SeqCst);
        self.virtq.avail.submit_chain(0);
        let last_seen = self.virtq.used.idx();
        // Kick.
        self.virtq.kick(0);
        // FIXME: spin for now. When supporting neither I/O apic or msi-x, use
        // interrupt.
        loop {
            fence(Ordering::SeqCst);
            if last_seen != self.virtq.used.idx() {
                break;
            }
        }
        self.virtq.used[last_seen as usize].len as usize
    }
}
