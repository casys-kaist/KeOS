//! Extended Page Table.
//!
//! ## Background
//! When multiple virtual machines are running on a single physical machine, the
//! hypervisor must need to have a method for mapping virtual machine's memory
//! addresses to the physical memory addresses of the host memory.
//! For this purpose, the hypervisor emulates the guest page table through
//! Shadow Paging. However, virtualizing memory access by shadow paging requires
//! processing all requests for virtual address translation demanded by all
//! virtual machine OS kernels (such as CR3 register access, page table
//! modifications, and VA-PA translation via MMU). EPT allows virtual machines
//! to have their page tables, which map virtual addresses to physical
//! addresses, while also allowing the hypervisor to maintain its own page table
//! for the host machine.
//!
//! ## Tasks
//! In this project, you are requested to implement of the Extended Page Table
//! for the gKeOS operating system. To manage and translate the guest physical
//! address to the host physical address, [`simple_ept_vm`] uses the implemented
//! EPT functionalities in this project. The main concept of this project is
//! similar to the page table implementations of Project 1. You have to
//! implement [`ExtendedPageTable::map`], [`ExtendedPageTable::unmap`] and
//! [`ExtendedPageTable::walk`] to be used for managing extended page table.
//! In contrast to the page table implementation from Project 1, EPT determines
//! the presence of an entry by examining the presence of flags in page table
//! entries. Stated differently, if there are no flags present in an EPT entry,
//! this indicates that the physical address referenced by the entry is not
//! valid (i.e., it is set to None). It is important to account for huge pages
//! in the address translation process [`kev::Probe::gpa2hpa`], as there are
//! instances where the allocation of huge pages cannot be avoided in x86 at the
//! initial boot time.
//!
//! [`simple_ept_vm`]: crate::simple_ept_vm
use alloc::boxed::Box;
use core::ops::{Deref, DerefMut};
use keos::{
    addressing::{Kva, PAGE_MASK, PAGE_SHIFT, Pa},
    mm::{
        Page,
        page_table::{Pde, PdeFlags, Pdpe, PdpeFlags, Pml4e, Pml4eFlags, Pte, PteFlags},
    },
};
use kev::{
    vm::{Gpa, Gva},
    vmcs::ActiveVmcs,
};

#[derive(Debug, PartialEq, Eq)]
pub enum EptMappingError {
    /// Unaligned address
    Unaligned,
    /// Not exist
    NotExist,
    /// Has a duplicated mapping.
    Duplicated,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct EptPml4e(usize);

impl EptPml4e {
    /// Get a physical address pointed by this entry.
    #[inline]
    pub fn pa(&self) -> Option<Pa> {
        todo!()
    }

    /// Get a flags this entry.
    #[inline]
    pub const fn flags(&self) -> EptPml4eFlags {
        EptPml4eFlags::from_bits_truncate(self.0)
    }

    /// Set physical address of this entry.
    ///
    /// # WARNING
    /// Permission of this entry is not changed.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, EptMappingError> {
        let pa = pa.into_usize();
        if pa & 0xfff != 0 {
            Err(EptMappingError::Unaligned)
        } else {
            self.0 = pa | (self.0 & EptPml4eFlags::all().bits());
            Ok(self)
        }
    }

    /// Set a permission of this entry.
    #[inline]
    pub fn set_perm(&mut self, perm: EptPml4eFlags) -> &mut Self {
        self.0 = perm.bits() | (self.0 & !EptPml4eFlags::all().bits());
        self
    }

    /// Get a mutable reference of page directory pointer table pointed by this
    /// entry.
    #[inline]
    pub fn into_ept_pdp_mut(&mut self) -> Result<&mut [EptPdpe], EptMappingError> {
        let pa = self.pa().ok_or(EptMappingError::NotExist)?;
        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                pa.into_kva().into_usize() as *mut EptPdpe,
                512,
            ))
        }
    }

    /// Get a reference of page directory pointer table pointed by this entry.
    #[inline]
    pub fn into_ept_pdp(&self) -> Result<&[EptPdpe], EptMappingError> {
        let pa = self.pa().ok_or(EptMappingError::NotExist)?;
        unsafe {
            Ok(core::slice::from_raw_parts(
                pa.into_kva().into_usize() as *const EptPdpe,
                512,
            ))
        }
    }
}

bitflags::bitflags! {
    /// Table 28-1. Format of an EPT PML4 Entry (PML4E) that References an EPT Page-Directory-Pointer Table
    pub struct EptPml4eFlags: usize {
        /// indicates whether reads are allowed from the 512-GByte region controlled by this entry
        const READ = 1 << 0;
        /// indicates whether writes are allowed to the 512-GByte region controlled by this entry
        const WRITE = 1 << 1;
        /// If the “mode-based execute control for EPT” VM-execution control is 0, execute access; indicates whether instruction
        /// fetches are allowed from the 512-GByte region controlled by this entry
        ///
        /// If that control is 1, execute access for supervisor-mode linear addresses; indicates whether instruction fetches are
        /// allowed from supervisor-mode linear addresses in the 512-GByte region controlled by this entry
        const EXECUTE = 1 << 2;
        /// If bit 6 of EPTP is 1, accessed flag for EPT; indicates whether software has accessed
        /// the 512-GByte region controlled by this entry (see Section 28.3.5). Ignored if bit 6 of EPTP is 0
        const ACCESSED = 1 << 8;
        /// If the “mode-based execute control for EPT” VM-execution control is 1, indicates whether instruction
        /// fetches are allowed from user-mode linear addresses in the 512-GByte region controlled by this entry.
        ///
        /// If that control is 0, this bit is ignored.
        const USER_EXECUTE = 1 << 10;

        const FULL = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits();
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct EptPdpe(usize);

impl EptPdpe {
    /// Get a physical address pointed by this entry.
    #[inline]
    pub fn pa(&self) -> Option<Pa> {
        todo!()
    }

    /// Get a flags this entry.
    #[inline]
    pub const fn flags(&self) -> EptPdpeFlags {
        EptPdpeFlags::from_bits_truncate(self.0)
    }

    /// Set physical address of this entry.
    ///
    /// # WARNING
    /// Permission of this entry is not changed.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, EptMappingError> {
        let pa = pa.into_usize();
        if pa & 0xfff != 0 {
            Err(EptMappingError::Unaligned)
        } else {
            self.0 = pa | (self.0 & EptPdpeFlags::all().bits());
            Ok(self)
        }
    }

    /// Set a permission of this entry.
    #[inline]
    pub fn set_perm(&mut self, perm: EptPdpeFlags) -> &mut Self {
        self.0 = perm.bits() | (self.0 & !EptPdpeFlags::all().bits());
        self
    }

    /// Get a mutable reference of page directory pointed by this entry.
    #[inline]
    pub fn into_ept_pd_mut(&mut self) -> Result<&mut [EptPde], EptMappingError> {
        let pa = self.pa().ok_or(EptMappingError::NotExist)?;
        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                pa.into_kva().into_usize() as *mut EptPde,
                512,
            ))
        }
    }

    /// Get a reference of page directory pointed by this entry.
    #[inline]
    pub fn into_ept_pd(&self) -> Result<&[EptPde], EptMappingError> {
        let pa = self.pa().ok_or(EptMappingError::NotExist)?;
        unsafe {
            Ok(core::slice::from_raw_parts(
                pa.into_kva().into_usize() as *const EptPde,
                512,
            ))
        }
    }
}

bitflags::bitflags! {
    /// Table 28-3. Format of an EPT Page-Directory-Pointer-Table Entry (PDPTE) that References an EPT Page Directory
    pub struct EptPdpeFlags: usize {
        /// indicates whether reads are allowed from the 1-GByte region controlled by this entry
        const READ = 1 << 0;
        /// indicates whether writes are allowed to the 1-GByte region controlled by this entry
        const WRITE = 1 << 1;
        /// If the “mode-based execute control for EPT” VM-execution control is 0, execute access; indicates whether instruction
        /// fetches are allowed from the 1-GByte region controlled by this entry
        ///
        /// If that control is 1, execute access for supervisor-mode linear addresses; indicates whether instruction fetches are
        /// allowed from supervisor-mode linear addresses in the 1-GByte region controlled by this entry
        const EXECUTE = 1 << 2;
        /// If bit 6 of EPTP is 1, accessed flag for EPT; indicates whether software has accessed
        /// the 1-GByte region controlled by this entry (see Section 28.3.5). Ignored if bit 6 of EPTP is 0
        const ACCESSED = 1 << 8;
        /// If the “mode-based execute control for EPT” VM-execution control is 1, indicates whether instruction
        /// fetches are allowed from user-mode linear addresses in the 1-GByte region controlled by this entry.
        ///
        /// If that control is 0, this bit is ignored.
        const USER_EXECUTE = 1 << 10;

        const FULL = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits();
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct EptPde(usize);

impl EptPde {
    /// Get a physical address pointed by this entry.
    #[inline]
    pub fn pa(&self) -> Option<Pa> {
        todo!()
    }

    /// Get a flags this entry.
    #[inline]
    pub const fn flags(&self) -> EptPdeFlags {
        EptPdeFlags::from_bits_truncate(self.0)
    }

    /// Set physical address of this entry.
    ///
    /// # WARNING
    /// Permission of this entry is not changed.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, EptMappingError> {
        let pa = pa.into_usize();
        if pa & 0xfff != 0 {
            Err(EptMappingError::Unaligned)
        } else {
            self.0 = pa | (self.0 & EptPdeFlags::all().bits());
            Ok(self)
        }
    }

    /// Set a permission of this entry.
    #[inline]
    pub fn set_perm(&mut self, perm: EptPdeFlags) -> &mut Self {
        self.0 = perm.bits() | (self.0 & !EptPdeFlags::all().bits());
        self
    }

    /// Get a mutable reference of page table pointed by this entry.
    #[inline]
    pub fn into_ept_pt_mut(&mut self) -> Result<&mut [EptPte], EptMappingError> {
        let pa = self.pa().ok_or(EptMappingError::NotExist)?;
        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                pa.into_kva().into_usize() as *mut EptPte,
                512,
            ))
        }
    }

    /// Get a reference of page table pointed by this entry.
    #[inline]
    pub fn into_ept_pt(&self) -> Result<&[EptPte], EptMappingError> {
        let pa = self.pa().ok_or(EptMappingError::NotExist)?;
        unsafe {
            Ok(core::slice::from_raw_parts(
                pa.into_kva().into_usize() as *const EptPte,
                512,
            ))
        }
    }
}

bitflags::bitflags! {
    /// Table 28-5. Format of an EPT Page-Directory Entry (PDE) that References an EPT Page Table
    pub struct EptPdeFlags: usize {
        /// indicates whether reads are allowed from the 2-MByte region controlled by this entry
        const READ = 1 << 0;
        /// indicates whether writes are allowed to the 2-MByte region controlled by this entry
        const WRITE = 1 << 1;
        /// If the “mode-based execute control for EPT” VM-execution control is 0, execute access; indicates whether instruction
        /// fetches are allowed from the 2-MByte region controlled by this entry
        ///
        /// If that control is 1, execute access for supervisor-mode linear addresses; indicates whether instruction fetches are
        /// allowed from supervisor-mode linear addresses in the 2-MByte region controlled by this entry
        const EXECUTE = 1 << 2;
        /// If bit 6 of EPTP is 1, accessed flag for EPT; indicates whether software has accessed
        /// the 2-MByte region controlled by this entry (see Section 28.3.5). Ignored if bit 6 of EPTP is 0
        const ACCESSED = 1 << 8;
        /// If the “mode-based execute control for EPT” VM-execution control is 1, indicates whether instruction
        /// fetches are allowed from user-mode linear addresses in the 2-MByte region controlled by this entry.
        ///
        /// If that control is 0, this bit is ignored.
        const USER_EXECUTE = 1 << 10;

        const FULL = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits();
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct EptPte(usize);

impl EptPte {
    /// Get a physical address pointed by this entry.
    #[inline]
    pub fn pa(&self) -> Option<Pa> {
        todo!()
    }

    /// Get a flags this entry.
    #[inline]
    pub const fn flags(&self) -> EptPteFlags {
        EptPteFlags::from_bits_truncate(self.0)
    }

    /// Set physical address of this entry.
    ///
    /// # WARNING
    /// Permission of this entry is not changed.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, EptMappingError> {
        let pa = pa.into_usize();
        if pa & 0xfff != 0 {
            Err(EptMappingError::Unaligned)
        } else {
            self.0 = pa | (self.0 & EptPteFlags::all().bits());
            Ok(self)
        }
    }

    /// Set a permission of this entry.
    #[inline]
    pub fn set_perm(&mut self, perm: EptPteFlags) -> &mut Self {
        self.0 = perm.bits() | (self.0 & !EptPteFlags::all().bits());
        self
    }
}

bitflags::bitflags! {
    /// Table 29-6. Format of an EPT Page-Table Entry that Maps a 4-KByte Page
    pub struct EptPteFlags: usize {
        /// indicates whether reads are allowed from the 4-KByte page referenced by this entry
        const READ = 1 << 0;
        /// indicates whether writes are allowed to the 4-KByte page referenced by this entry
        const WRITE = 1 << 1;
        /// If the “mode-based execute control for EPT” VM-execution control is 0, execute access;
        /// indicates whether instruction fetches are allowed from the 4-KByte page controlled by this entry
        ///
        /// If that control is 1, execute access for supervisor-mode linear addresses; indicates whether instruction fetches are
        /// allowed from supervisor-mode linear addresses in the 4-KByte page controlled by this entry
        const EXECUTE = 1 << 2;
        // bit 5-3. EPT memory type for this 4-KByte page (see Section 28.3.7)
        #[doc(hidden)]
        const BIT3 = 1 << 3;
        #[doc(hidden)]
        const BIT4 = 1 << 4;
        #[doc(hidden)]
        const BIT5 = 1 << 5;
        // bit 6. Ignore PAT memory type for this 4-KByte page (see Section 28.3.7)
        #[doc(hidden)]
        const BIT6 = 1 << 6;

        /// indicates whether software has accessed the 4-KByte page referenced by this entry (see Section 28.3.5).
        /// Ignored if bit 6 of EPTP is 0
        const ACCESSED = 1 << 8;
        /// indicates whether software has written to the 4-KByte page referenced by this entry (see Section 28.3.5).
        /// Ignored if bit 6 of EPTP is 0
        const DIRTY = 1 << 9;
        /// Verify guest paging. If the “guest-paging verification” VM-execution control is 1, indicates limits on the guest paging
        /// structures used to access the 4-KByte page controlled by this entry (see Section 28.3.3.2).
        /// If that control is 0, this bit is ignored.
        const VERIFY = 1 << 57;
        /// If the “EPT paging-write control” VM-execution control is 1, indicates that guest paging may
        /// update the 4-KByte page controlled by this entry (see Section 28.3.3.2).
        /// If that control is 0, this bit is ignored.
        const PAGING_WRITE_ACCESS = 1 << 58;
        /// If bit 7 of EPTP is 1, indicates whether supervisor shadow stack accesses are allowed to
        /// guest-physical addresses in the 4-KByte page mapped by this entry (see Section 28.3.3.2).
        /// Ignored if bit 7 of EPTP is 0
        const SUPERVISOR_SHADOW_STACK = 1 << 60;
        /// If the “sub-page write permissions for EPT” VM-execution control is 1,
        /// writes to individual 128-byte regions of the 4-KByte page referenced by
        /// this entry may be allowed even if the page would normally not be writable (see Section 28.3.4).
        /// If “sub-page write permissions for EPT” VM-execution control is 0, this bit is ignored.
        const SUBPAGE_WRITE_PERMISSION = 1 << 61;
        ///  If the “EPT-violation #VE” VM-execution control is 1, EPT violations caused by accesses to
        /// this page are convertible to virtualization exceptions only if this bit is 0 (see Section 25.5.7.1).
        /// If “EPT-violation #VE” VMexecution control is 0, this bit is ignored.
        const SUPPRESS_VE = 1 << 63;

        const FULL = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits();
    }
}

bitflags::bitflags! {
    /// Possible memory permissions.
    pub struct Permission: usize {
        /// Page is readable.
        const READ = 1 << 0;
        /// Page is writable.
        const WRITE = 1 << 1;
        /// Page is executable.
        const EXECUTABLE = 1 << 2;
    }
}

// Make ept align to 4096.
#[repr(align(4096))]
struct Inner([EptPml4e; 512]);
impl Deref for Inner {
    type Target = [EptPml4e; 512];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Inner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Second level page table that holds guest-physical to host-physical mapping.
pub struct ExtendedPageTable(Box<Inner>);

impl Default for ExtendedPageTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtendedPageTable {
    pub fn new() -> Self {
        Self(unsafe { Box::new_zeroed().assume_init() })
    }

    pub fn pa(&self) -> Pa {
        Kva::new(self.0.as_ref().as_ptr() as usize)
            .unwrap()
            .into_pa()
    }

    /// Map `pg` into `va` with permission `perm`.
    pub fn map(&mut self, gpa: Gpa, pg: Page, perm: Permission) -> Result<(), EptMappingError> {
        unsafe { self.do_map(gpa, pg.into_raw(), perm) }
    }

    /// Map `hpa` into `pa` with permission `perm`.
    ///
    /// # Safety
    /// The `Pa` must point the physical address of allocated page.
    pub unsafe fn do_map(
        &mut self,
        gpa: Gpa,
        hpa: Pa,
        perm: Permission,
    ) -> Result<(), EptMappingError> {
        // Hint: Use each flags's `FULL` to determine whether entry is allocated or not.
        todo!()
    }

    /// Unmap the `gpa` and returns `Page` that was mapped to `gpa`.
    pub fn unmap(&mut self, gpa: Gpa) -> Result<Page, EptMappingError> {
        // Hint: Use `Page::from_pa()`.
        todo!()
    }

    /// Walk the extended page table and return corresponding eptpte of the
    /// `gpa` if exist.
    pub fn walk(&self, gpa: Gpa) -> Result<&EptPte, EptMappingError> {
        todo!()
    }
}

impl kev::Probe for ExtendedPageTable {
    fn gpa2hpa(&self, _vmcs: &ActiveVmcs, gpa: Gpa) -> Option<Pa> {
        todo!()
    }
    fn gva2hpa(&self, vmcs: &ActiveVmcs, gva: Gva) -> Option<Pa> {
        // Hint:
        //   - You should consider the 2M huge page.
        todo!()
    }
}
