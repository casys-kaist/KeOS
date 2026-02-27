//! Entries of Page Table and thier permissions.
use crate::{
    addressing::{Pa, Va},
    mm::{Page, tlb::TlbIpi},
    sync::atomic::AtomicUsize,
};
use abyss::{MAX_CPU, x86_64::Cr3};
use alloc::boxed::Box;
use core::ops::Deref;

bitflags::bitflags! {
    /// Flags for pml4e.
    pub struct Pml4eFlags: usize {
        /// Present; must be 1 to reference a page-directory-pointer table
        const P = 1 << 0;
        /// Read/write; if 0, writes may not be allowed to the 512-GByte region controlled by this entry (see Section 4.6).
        const RW = 1 << 1;
        /// User/supervisor; if 0, user-mode accesses are not allowed to the 512-GByte region controlled by this entry (see Section 4.6)
        const US = 1 << 2;
        /// Page-level write-through; indirectly determines the memory type used to access the page-directory-pointer table referenced by this entry (see Section 4.9.2)
        const PWT = 1 << 3;
        /// Page-level cache disable; indirectly determines the memory type used to access the page-directory-pointer table referenced by this entry (see Section 4.9.2)
        const PCD = 1 << 4;
        /// Accessed; indicates whether this entry has been used for linear-address translation (see Section 4.8)
        const A = 1 << 5;
        #[doc(hidden)] const _IGN_6 = 1 << 6;
        #[doc(hidden)] const _REV_0 = 1 << 7;
        #[doc(hidden)] const _IGN_8 = 1 << 8;
        #[doc(hidden)] const _IGN_9 = 1 << 9;
        #[doc(hidden)] const _IGN_10 = 1 << 10;
        /// For ordinary paging, ignored; for HLAT paging, restart (if 1, linear-address translation is restarted with ordinary paging)
        const R = 1 << 11;
        #[doc(hidden)] const _IGN_52 = 1 << 52;
        #[doc(hidden)] const _IGN_53 = 1 << 53;
        #[doc(hidden)] const _IGN_54 = 1 << 54;
        #[doc(hidden)] const _IGN_55 = 1 << 55;
        #[doc(hidden)] const _IGN_56 = 1 << 56;
        #[doc(hidden)] const _IGN_57 = 1 << 57;
        #[doc(hidden)] const _IGN_58 = 1 << 58;
        #[doc(hidden)] const _IGN_59 = 1 << 59;
        #[doc(hidden)] const _IGN_60 = 1 << 60;
        #[doc(hidden)] const _IGN_61 = 1 << 61;
        #[doc(hidden)] const _IGN_62 = 1 << 62;
        /// If IA32_EFER.NXE = 1, execute-disable (if 1, instruction fetches are not allowed from the 512-GByte region controlled by this entry; see Section 4.6); otherwise, reserved (must be 0)
        const XD = 1 << 63;
    }
}

/// Page Map Level 4 Entry (PML4E).
///
/// This struct represents a **Page Map Level 4 Entry** (PML4E), which is the
/// top-level entry in the 4-level page table system used in x86_64
/// architecture. A PML4E is the highest-level entry in the virtual memory
/// hierarchy and points to a **Page Directory Pointer Table** (PDP) or a
/// higher-level page table that contains further mappings for virtual to
/// physical memory.
///
/// The [`Pml4e`] struct provides methods for working with the physical address
/// and flags associated with a PML4E, allowing manipulation of page tables in
/// the virtual memory system.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Pml4e(pub usize);

impl core::fmt::Debug for Pml4e {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(pa) = self.pa() {
            write!(f, "Pml4e({:016x}, {:?})", pa.into_usize(), self.flags())
        } else {
            write!(f, ".")
        }
    }
}

impl Pml4e {
    /// Get the physical address pointed to by this entry.
    ///
    /// This function checks whether the PML4 entry is **present** (i.e., if the
    /// "P" flag is set in the entry). If the entry is present, it extracts
    /// the physical address by clearing the flags from the entry.
    ///
    /// # Returns
    /// - `Some(Pa)` if the PML4E is present, containing the physical address.
    /// - `None` if the PML4E is not present (i.e., the "P" flag is not set).
    #[inline]
    pub const fn pa(&self) -> Option<Pa> {
        if self.flags().contains(Pml4eFlags::P) {
            Pa::new(self.0 & !Pml4eFlags::all().bits())
        } else {
            None
        }
    }

    /// Get the flags associated with this entry.
    ///
    /// This function extracts the flags from the PML4E, which may indicate
    /// whether the page map level entry is present,
    /// writable, user-accessible, etc.
    ///
    /// # Returns
    /// A [`Pml4eFlags`] value representing the flags associated with this
    /// entry.
    #[inline]
    pub const fn flags(&self) -> Pml4eFlags {
        Pml4eFlags::from_bits_truncate(self.0)
    }

    /// Set the physical address for this entry.
    ///
    /// This method updates the physical address of the PML4E while preserving
    /// the current flags (e.g., read/write permissions). It ensures that
    /// the provided physical address is aligned to a 4K boundary (the page
    /// size), as required by the architecture.
    ///
    /// # Parameters
    /// - `pa`: The new physical address to set for the entry.
    ///
    /// # Returns
    /// - `Ok(&mut Self)` if the address is valid and the update is successful.
    /// - `Err(PageTableMappingError::Unaligned)` if the provided physical
    ///   address is not aligned.
    ///
    /// # Warning
    /// This operation does not modify the flags of the entry.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, PageTableMappingError> {
        let pa = { pa.into_usize() };
        if pa & 0xfff != 0 {
            Err(PageTableMappingError::Unaligned)
        } else {
            self.0 = pa | self.flags().bits() | Pml4eFlags::P.bits();
            Ok(self)
        }
    }

    /// Set the flags for this entry.
    ///
    /// This method allows you to update the flags associated with the PML4E
    /// without modifying the physical address. It combines the current
    /// physical address with the new flags and sets the updated value back into
    /// the entry.
    ///
    /// # Parameters
    /// - `perm`: The new set of flags to assign to the entry.
    ///
    /// # Returns
    /// A mutable reference to `self`, allowing for method chaining.
    #[inline]
    pub fn set_flags(&mut self, perm: Pml4eFlags) -> &mut Self {
        self.0 = self.pa().map(|n| n.into_usize()).unwrap_or(0) | perm.bits();
        self
    }

    /// Clears the entry.
    ///
    /// This method removes any previously set physical address and flags from
    /// the entry. If the entry contained a valid physical address before
    /// being cleared, that address is returned.
    ///
    /// # Returns
    /// - `Some(Pa)`: The physical address that was previously stored in the
    ///   entry, if it existed.
    /// - `None`: If the entry did not contain a valid physical address.
    #[inline]
    pub fn clear(&mut self) -> Option<Pa> {
        self.pa().inspect(|_| {
            self.0 = 0;
        })
    }

    /// Get a mutable reference to the page directory pointer table pointed to
    /// by this entry.
    ///
    /// This method retrieves a mutable reference to the page directory pointer
    /// table (PDP) that this PML4E points to, assuming that the entry is
    /// present (i.e., the "P" flag is set).
    ///
    /// # Returns
    /// - `Ok(&mut [Pdpe])` if the page directory pointer table is valid,
    ///   represented as a mutable slice of `Pdpe` (page directory pointer
    ///   entries).
    /// - `Err(PageTableMappingError::NotExist)` if the PML4E is not present or
    ///   invalid.
    ///
    /// # Safety
    /// This operation assumes that the physical address of the page directory
    /// pointer table is valid and properly aligned.
    #[inline]
    pub fn into_pdp_mut(&mut self) -> Result<&mut [Pdpe], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(Pml4eFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                pa.into_kva().into_usize() as *mut Pdpe,
                512,
            ))
        }
    }

    /// Get a reference to the page directory pointer table pointed to by this
    /// entry.
    ///
    /// This method retrieves an immutable reference to the page directory
    /// pointer table (PDP) that this PML4E points to, assuming that the
    /// entry is present (i.e., the "P" flag is set).
    ///
    /// # Returns
    /// - `Ok(&[Pdpe])` if the page directory pointer table is valid,
    ///   represented as an immutable slice of `Pdpe` (page directory pointer
    ///   entries).
    /// - `Err(PageTableMappingError::NotExist)` if the PML4E is not present or
    ///   invalid.
    ///
    /// # Safety
    /// This operation assumes that the physical address of the page directory
    /// pointer table is valid and properly aligned.
    #[inline]
    pub fn into_pdp(&self) -> Result<&[Pdpe], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(Pml4eFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts(
                pa.into_kva().into_usize() as *const Pdpe,
                512,
            ))
        }
    }
}

bitflags::bitflags! {
    /// Flags for pdpe.
    pub struct PdpeFlags: usize {
        /// Present; must be 1 to reference a page directory
        const P = 1 << 0;
        /// Read/write; if 0, writes may not be allowed to the 1-GByte region controlled by this entry (see Section 4.6)
        const RW = 1 << 1;
        /// User/supervisor; if 0, user-mode accesses are not allowed to the 1-GByte region controlled by this entry (see Section 4.6)
        const US = 1 << 2;
        /// Page-level write-through; indirectly determines the memory type used to access the page directory referenced by this entry (see Section 4.9.2)
        const PWT = 1 << 3;
        /// Page-level cache disable; indirectly determines the memory type used to access the page directory referenced by this entry (see Section 4.9.2)
        const PCD = 1 << 4;
        /// Accessed; indicates whether this entry has been used for linear-address translation (see Section 4.8)
        const A = 1 << 5;
        #[doc(hidden)] const _IGN_6 = 1 << 6;
        #[doc(hidden)] const _REV_0 = 1 << 7;
        #[doc(hidden)] const _IGN_8 = 1 << 8;
        #[doc(hidden)] const _IGN_9 = 1 << 9;
        #[doc(hidden)] const _IGN_10 = 1 << 10;
        /// For ordinary paging, ignored; for HLAT paging, restart (if 1, linear-address translation is restarted with ordinary paging)
        const R = 1 << 11;
        #[doc(hidden)] const _IGN_52 = 1 << 52;
        #[doc(hidden)] const _IGN_53 = 1 << 53;
        #[doc(hidden)] const _IGN_54 = 1 << 54;
        #[doc(hidden)] const _IGN_55 = 1 << 55;
        #[doc(hidden)] const _IGN_56 = 1 << 56;
        #[doc(hidden)] const _IGN_57 = 1 << 57;
        #[doc(hidden)] const _IGN_58 = 1 << 58;
        #[doc(hidden)] const _IGN_59 = 1 << 59;
        #[doc(hidden)] const _IGN_60 = 1 << 60;
        #[doc(hidden)] const _IGN_61 = 1 << 61;
        #[doc(hidden)] const _IGN_62 = 1 << 62;
        /// If IA32_EFER.NXE = 1, execute-disable (if 1, instruction fetches are not allowed from the 1-GByte region controlled by this entry; see Section 4.6); otherwise, reserved (must be 0)
        const XD = 1 << 63;
    }
}

/// Page Directory Pointer Table Entry (PDPE).
///
/// This struct represents a **Page Directory Pointer Table Entry** (PDPE), the
/// entry of second-level table, in the 4-level page table system for x86_64
/// architecture. A PDPE is the second-level entry in the virtual memory
/// hierarchy, directly pointing to a **Page Directory** (PDE) or a higher-level
/// page table that contains further mappings for virtual to physical memory.
///
/// The [`Pdpe`] struct provides methods for working with the physical address
/// and flags associated with a PDPE, allowing the manipulation of page tables
/// in the virtual memory system.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Pdpe(pub usize);

impl Pdpe {
    /// Get the physical address pointed to by this entry.
    ///
    /// This function checks whether the page directory pointer table entry is
    /// **present** (i.e., if the "P" flag is set in the entry).
    /// If the entry is present, it extracts the physical address by clearing
    /// the flags from the entry.
    ///
    /// # Returns
    /// - `Some(Pa)` if the PDPE is present, containing the physical address.
    /// - `None` if the PDPE is not present (i.e., the "P" flag is not set).
    #[inline]
    pub const fn pa(&self) -> Option<Pa> {
        if self.flags().contains(PdpeFlags::P) {
            Pa::new(self.0 & !PdpeFlags::all().bits())
        } else {
            None
        }
    }

    /// Get the flags associated with this entry.
    ///
    /// This function extracts the flags from the PDPE, which may indicate
    /// whether the page directory pointer table entry is present, writable,
    /// user-accessible, etc.
    ///
    /// # Returns
    /// A [`PdpeFlags`] value representing the flags associated with this entry.
    #[inline]
    pub const fn flags(&self) -> PdpeFlags {
        PdpeFlags::from_bits_truncate(self.0)
    }

    /// Set the physical address for this entry.
    ///
    /// This method updates the physical address of the PDPE while preserving
    /// the current flags (e.g., read/write permissions). It ensures that
    /// the provided physical address is aligned to a 4K boundary (the page
    /// size), as required by the architecture.
    ///
    /// # Parameters
    /// - `pa`: The new physical address to set for the entry.
    ///
    /// # Returns
    /// - `Ok(&mut Self)` if the address is valid and the update is successful.
    /// - `Err(PageTableMappingError::Unaligned)` if the provided physical
    ///   address is not aligned.
    ///
    /// # Warning
    /// This operation does not modify the flags of the entry.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, PageTableMappingError> {
        let pa = { pa.into_usize() };
        if pa & 0xfff != 0 {
            Err(PageTableMappingError::Unaligned)
        } else {
            self.0 = pa | self.flags().bits() | PdpeFlags::P.bits();
            Ok(self)
        }
    }

    /// Set the flags for this entry.
    ///
    /// This method allows you to update the flags associated with the PDPE
    /// without modifying the physical address. It combines the current
    /// physical address with the new flags and sets the updated value back into
    /// the entry.
    ///
    /// # Parameters
    /// - `perm`: The new set of flags to assign to the entry.
    ///
    /// # Returns
    /// A mutable reference to `self`, allowing for method chaining.
    #[inline]
    pub fn set_flags(&mut self, perm: PdpeFlags) -> &mut Self {
        self.0 = self.pa().map(|n| n.into_usize()).unwrap_or(0) | perm.bits();
        self
    }

    /// Clears the entry.
    ///
    /// This method removes any previously set physical address and flags from
    /// the entry. If the entry contained a valid physical address before
    /// being cleared, that address is returned.
    ///
    /// # Returns
    /// - `Some(Pa)`: The physical address that was previously stored in the
    ///   entry, if it existed.
    /// - `None`: If the entry did not contain a valid physical address.
    #[inline]
    pub fn clear(&mut self) -> Option<Pa> {
        self.pa().inspect(|_| {
            self.0 = 0;
        })
    }

    /// Get a mutable reference to the page directory pointed to by this entry.
    ///
    /// This method retrieves a mutable reference to the page directory that
    /// this PDPE points to, assuming that the entry is present (i.e., the
    /// "P" flag is set).
    ///
    /// # Returns
    /// - `Ok(&mut [Pde])` if the page directory is valid, represented as a
    ///   mutable slice of `Pde` (page directory entries).
    /// - `Err(PageTableMappingError::NotExist)` if the PDPE is not present or
    ///   invalid.
    ///
    /// # Safety
    /// This operation assumes that the physical address of the page directory
    /// is valid and properly aligned.
    #[inline]
    pub fn into_pd_mut(&mut self) -> Result<&mut [Pde], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(PdpeFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                pa.into_kva().into_usize() as *mut Pde,
                512,
            ))
        }
    }

    /// Get a reference to the page directory pointed to by this entry.
    ///
    /// This method retrieves an immutable reference to the page directory that
    /// this PDPE points to, assuming that the entry is present (i.e., the
    /// "P" flag is set).
    ///
    /// # Returns
    /// - `Ok(&[Pde])` if the page directory is valid, represented as an
    ///   immutable slice of `Pde` (page directory entries).
    /// - `Err(PageTableMappingError::NotExist)` if the PDPE is not present or
    ///   invalid.
    ///
    /// # Safety
    /// This operation assumes that the physical address of the page directory
    /// is valid and properly aligned.
    #[inline]
    pub fn into_pd(&self) -> Result<&[Pde], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(PdpeFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts(
                pa.into_kva().into_usize() as *const Pde,
                512,
            ))
        }
    }
}

/// Page Directory Entry (PDE).
///
/// This struct represents a **Page Directory Entry** (PDE), entry of
/// third-level table, in the 4-level page table system for x86_64 architecture.
/// A page directory entry typically holds the information about the physical
/// address of a page directory or a page table, along with various flags.
/// In a paging system, a PDE points to a page table, which in turn contains the
/// actual page table entries (PTEs) that map virtual addresses to physical
/// addresses.
///
/// The [`Pde`] struct in this code provides access to these fields and
/// operations on them. Each entry corresponds to a page directory in the
/// virtual memory hierarchy and is used by the kernel to map higher-level
/// virtual addresses to lower-level page table entries.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Pde(pub usize);

impl Pde {
    /// Get the physical address pointed to by this entry.
    ///
    /// This function checks whether the page directory entry is **present**
    /// (i.e., if the "P" flag is set in the entry). If the page directory
    /// entry is present, it extracts the physical address by clearing the flags
    /// from the entry.
    ///
    /// # Returns
    /// - `Some(Pa)` if the page directory entry is present, containing the
    ///   physical address.
    /// - `None` if the page directory entry is not present (i.e., the "P" flag
    ///   is not set).
    #[inline]
    pub const fn pa(&self) -> Option<Pa> {
        if self.flags().contains(PdeFlags::P) {
            Pa::new(self.0 & !PdeFlags::all().bits())
        } else {
            None
        }
    }

    /// Get the flags associated with this entry.
    ///
    /// This function extracts the flags from the page directory entry, which
    /// may indicate whether the page directory is present,
    /// writable, user-accessible, etc.
    ///
    /// # Returns
    /// A [`PdeFlags`] value representing the flags associated with this entry.
    #[inline]
    pub const fn flags(&self) -> PdeFlags {
        PdeFlags::from_bits_truncate(self.0)
    }

    /// Set the physical address for this entry.
    ///
    /// This method updates the physical address of the page directory entry
    /// while preserving the current flags (e.g., read/write permissions).
    /// It checks that the provided physical address is aligned to a 4K boundary
    /// (the page size), as required by the architecture.
    ///
    /// # Parameters
    /// - `pa`: The new physical address to set for the entry.
    ///
    /// # Returns
    /// - `Ok(&mut Self)` if the address is valid and the update is successful.
    /// - `Err(PageTableMappingError::Unaligned)` if the provided physical
    ///   address is not aligned.
    ///
    /// # Warning
    /// This operation does not modify the flags of the entry.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, PageTableMappingError> {
        let pa = pa.into_usize();
        if pa & 0xfff != 0 {
            Err(PageTableMappingError::Unaligned)
        } else {
            self.0 = pa | self.flags().bits() | PdeFlags::P.bits();
            Ok(self)
        }
    }

    /// Set the flags for this entry.
    ///
    /// This method allows you to update the flags associated with the page
    /// directory entry without modifying the physical address. It combines
    /// the current physical address with the new flags and sets the updated
    /// value back into the entry.
    ///
    /// # Parameters
    /// - `perm`: The new set of flags to assign to the entry.
    ///
    /// # Returns
    /// A mutable reference to `self`, allowing for method chaining.
    #[inline]
    pub fn set_flags(&mut self, perm: PdeFlags) -> &mut Self {
        self.0 = self.pa().map(|n| n.into_usize()).unwrap_or(0) | perm.bits();
        self
    }

    /// Clears the entry.
    ///
    /// This method removes any previously set physical address and flags from
    /// the entry. If the entry contained a valid physical address before
    /// being cleared, that address is returned.
    ///
    /// # Returns
    /// - `Some(Pa)`: The physical address that was previously stored in the
    ///   entry, if it existed.
    /// - `None`: If the entry did not contain a valid physical address.
    #[inline]
    pub fn clear(&mut self) -> Option<Pa> {
        self.pa().inspect(|_| {
            self.0 = 0;
        })
    }

    /// Get a mutable reference to the page table pointed to by this entry.
    ///
    /// This method retrieves a mutable reference to the page table that this
    /// page directory entry points to, assuming that the entry is present
    /// (i.e., the "P" flag is set).
    ///
    /// # Returns
    /// - `Ok(&mut [Pte])` if the page table is valid, represented as a mutable
    ///   slice of `Pte` (page table entries).
    /// - `Err(PageTableMappingError::NotExist)` if the page directory entry is
    ///   not present or invalid.
    ///
    /// # Safety
    /// This operation assumes that the physical address of the page table is
    /// valid and properly aligned.
    #[inline]
    pub fn into_pt_mut(&mut self) -> Result<&mut [Pte], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(PdeFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts_mut(
                pa.into_kva().into_usize() as *mut Pte,
                512,
            ))
        }
    }

    /// Get a reference to the page table pointed to by this entry.
    ///
    /// This method retrieves an immutable reference to the page table that this
    /// page directory entry points to, assuming that the entry is present
    /// (i.e., the "P" flag is set).
    ///
    /// # Returns
    /// - `Ok(&[Pte])` if the page table is valid, represented as an immutable
    ///   slice of `Pte` (page table entries).
    /// - `Err(PageTableMappingError::NotExist)` if the page directory entry is
    ///   not present or invalid.
    ///
    /// # Safety
    /// This operation assumes that the physical address of the page table is
    /// valid and properly aligned.
    #[inline]
    pub fn into_pt(&self) -> Result<&[Pte], PageTableMappingError> {
        let pa = self.pa().ok_or(PageTableMappingError::NotExist)?;
        if !self.flags().contains(PdeFlags::P) {
            return Err(PageTableMappingError::NotExist);
        }
        unsafe {
            Ok(core::slice::from_raw_parts(
                pa.into_kva().into_usize() as *const Pte,
                512,
            ))
        }
    }
}

bitflags::bitflags! {
    /// Flags for pde.
    pub struct PdeFlags: usize {
        /// Present; must be 1 to reference a page table
        const P = 1 << 0;
        /// Read/write; if 0, writes may not be allowed to the 2-MByte region controlled by this entry (see Section 4.6)
        const RW = 1 << 1;
        /// User/supervisor; if 0, user-mode accesses are not allowed to the 2-MByte region controlled by this entry (see Section 4.6)
        const US = 1 << 2;
        /// Page-level write-through; indirectly determines the memory type used to access the page table referenced by this entry (see Section 4.9.2)
        const PWT = 1 << 3;
        /// Page-level cache disable; indirectly determines the memory type used to access the page table referenced by this entry (see Section 4.9.2)
        const PCD = 1 << 4;
        /// Accessed; indicates whether this entry has been used for linear-address translation (see Section 4.8)
        const A = 1 << 5;
        /// Page size; indicates whether this entry is 2M page.
        const PS = 1 << 7;
        #[doc(hidden)] const _IGN_6 = 1 << 6;
        #[doc(hidden)] const _REV_0 = 1 << 7;
        #[doc(hidden)] const _IGN_8 = 1 << 8;
        #[doc(hidden)] const _IGN_9 = 1 << 9;
        #[doc(hidden)] const _IGN_10 = 1 << 10;
        /// For ordinary paging, ignored; for HLAT paging, restart (if 1, linear-address translation is restarted with ordinary paging)
        const R = 1 << 11;
        #[doc(hidden)] const _IGN_52 = 1 << 52;
        #[doc(hidden)] const _IGN_53 = 1 << 53;
        #[doc(hidden)] const _IGN_54 = 1 << 54;
        #[doc(hidden)] const _IGN_55 = 1 << 55;
        #[doc(hidden)] const _IGN_56 = 1 << 56;
        #[doc(hidden)] const _IGN_57 = 1 << 57;
        #[doc(hidden)] const _IGN_58 = 1 << 58;
        #[doc(hidden)] const _IGN_59 = 1 << 59;
        #[doc(hidden)] const _IGN_60 = 1 << 60;
        #[doc(hidden)] const _IGN_61 = 1 << 61;
        #[doc(hidden)] const _IGN_62 = 1 << 62;
        /// If IA32_EFER.NXE = 1, execute-disable (if 1, instruction fetches are not allowed from the 2-MByte region controlled by this entry; see Section 4.6); otherwise, reserved (must be 0)
        const XD = 1 << 63;
    }
}

/// Page Table Entry (PTE).
///
/// This struct represents a Page Table Entry (PTE), the entry of last-level
/// table, in the 4-level page table system for x86_64 architecture.
/// A page table entry typically holds the information about the physical
/// address of a page and various control bits, such as flags indicating whether
/// the page is present, read/write, etc.
///
/// The [`Pte`] struct in this code provides access to these fields and
/// operations on them. Each entry corresponds to a single page in memory and is
/// used by the kernel to map virtual addresses to physical addresses in the
/// page table.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Pte(pub usize);

impl Pte {
    /// Get the physical address pointed to by this entry.
    ///
    /// This function checks whether the page is present (i.e., if the "P" flag
    /// is set in the entry). If the page is present, it extracts the
    /// physical address from the entry by clearing the flags bits.
    ///
    /// # Returns
    /// - `Some(Pa)` if the page is present, containing the physical address.
    /// - `None` if the page is not present (i.e., the "P" flag is not set).
    #[inline]
    pub const fn pa(&self) -> Option<Pa> {
        if self.flags().contains(PteFlags::P) {
            Pa::new(self.0 & !PteFlags::all().bits())
        } else {
            None
        }
    }

    /// Get the flags associated with this page table entry.
    ///
    /// This function extracts the flags from the entry. The flags represent
    /// various properties of the page, such as whether the page is present,
    /// read-only, user-accessible, etc.
    ///
    /// # Returns
    /// A [`PteFlags`] value representing the flags associated with this entry.
    #[inline]
    pub const fn flags(&self) -> PteFlags {
        PteFlags::from_bits_truncate(self.0)
    }

    /// Set the physical address for this entry.
    ///
    /// This method updates the physical address of the entry, preserving the
    /// current flags (e.g., read/write permissions). It checks that the
    /// physical address is aligned to a 4K boundary (the page size), as
    /// required by the architecture.
    ///
    /// # Safety
    /// You must invalidate the corresponding TLB Entry.
    ///
    /// # Parameters
    /// - `pa`: The new physical address to set for the entry.
    ///
    /// # Returns
    /// - `Ok(&mut Self)` if the address is valid and the update is successful.
    /// - `Err(PageTableMappingError::Unaligned)` if the provided physical
    ///   address is not aligned.
    ///
    /// # Warning
    /// This operation does not modify the flags of the entry.
    #[inline]
    pub fn set_pa(&mut self, pa: Pa) -> Result<&mut Self, PageTableMappingError> {
        let pa = pa.into_usize();
        if pa & 0xfff != 0 {
            Err(PageTableMappingError::Unaligned)
        } else {
            self.0 = pa | self.flags().bits() | PteFlags::P.bits();
            Ok(self)
        }
    }

    /// Set the flags for this entry.
    ///
    /// This method allows you to update the flags associated with the page.
    /// The physical address remains unchanged, but the permission settings
    /// (e.g., read/write, user/kernel) can be updated.
    ///  
    /// # Parameters
    /// - `perm`: The new set of flags to assign to the entry.
    ///
    /// # Returns
    /// A mutable reference to `self`, allowing for method chaining.
    ///   
    ///  # Safety
    /// You must invalidate the corresponding TLB Entry.
    #[inline]
    pub unsafe fn set_flags(&mut self, perm: PteFlags) -> &mut Self {
        self.0 = self.pa().map(|n| n.into_usize()).unwrap_or(0) | perm.bits();
        self
    }

    /// Clears the entry.
    ///
    /// This method removes any previously set physical address and flags from
    /// the entry. If the entry contained a valid physical address before
    /// being cleared, that address is returned.
    ///
    /// # Returns
    /// - `Some(Pa)`: The physical address that was previously stored in the
    ///   entry, if it existed.
    /// - `None`: If the entry did not contain a valid physical address.
    ///
    /// # Safety
    /// You must invalidate the corresponding TLB Entry.
    #[inline]
    pub unsafe fn clear(&mut self) -> Option<Pa> {
        self.pa().inspect(|_| {
            self.0 = 0;
        })
    }
}

bitflags::bitflags! {
    /// Flags for pte.
    pub struct PteFlags: usize {
        /// Present; must be 1 to map a 4-KByte page
        const P = 1 << 0;
        /// Read/write; if 0, writes may not be allowed to the 4-KByte page referenced by this entry (see Section 4.6)
        const RW = 1 << 1;
        /// User/supervisor; if 0, user-mode accesses are not allowed to the 4-KByte page referenced by this entry (see Section 4.6)
        const US = 1 << 2;
        /// Page-level write-through; indirectly determines the memory type used to access the 4-KByte page referenced by this entry (see Section 4.9.2)
        const PWT = 1 << 3;
        /// Page-level cache disable; indirectly determines the memory type used to access the 4-KByte page referenced by this entry (see Section 4.9.2)
        const PCD = 1 << 4;
        /// Accessed; indicates whether software has accessed the 4-KByte page referenced by this entry (see Section 4.8)
        const A = 1 << 5;
        /// Dirty; indicates whether software has written to the 4-KByte page referenced by this entry (see Section 4.8)
        const D = 1 << 6;
        /// Indirectly determines the memory type used to access the 4-KByte page referenced by this entry (see Section 4.9.2)
        const PAT = 1 << 7;
        /// Global; if CR4.PGE = 1, determines whether the translation is global (see Section 4.10); ignored otherwise
        const G = 1 << 8;
        #[doc(hidden)] const _IGN_9 = 1 << 9;
        #[doc(hidden)] const _IGN_10 = 1 << 10;
        /// For ordinary paging, ignored; for HLAT paging, restart (if 1, linear-address translation is restarted with ordinary paging)
        const R = 1 << 11;
        #[doc(hidden)] const _IGN_52 = 1 << 52;
        #[doc(hidden)] const _IGN_53 = 1 << 53;
        #[doc(hidden)] const _IGN_54 = 1 << 54;
        #[doc(hidden)] const _IGN_55 = 1 << 55;
        #[doc(hidden)] const _IGN_56 = 1 << 56;
        #[doc(hidden)] const _IGN_57 = 1 << 57;
        #[doc(hidden)] const _IGN_58 = 1 << 58;
        /// Protection key bit 0; if CR4.PKE = 1 or CR4.PKS = 1, this may control the page’s access rights (see Section 4.6.2); otherwise, it is ignored and not used to control access rights.
        const PK_0 = 1 << 59;
        /// Protection key bit 1; if CR4.PKE = 1 or CR4.PKS = 1, this may control the page’s access rights (see Section 4.6.2); otherwise, it is ignored and not used to control access rights.
        const PK_1 = 1 << 60;
        /// Protection key bit 2; if CR4.PKE = 1 or CR4.PKS = 1, this may control the page’s access rights (see Section 4.6.2); otherwise, it is ignored and not used to control access rights.
        const PK_2 = 1 << 61;
        /// Protection key bit 3; if CR4.PKE = 1 or CR4.PKS = 1, this may control the page’s access rights (see Section 4.6.2); otherwise, it is ignored and not used to control access rights.
        const PK_3 = 1 << 62;
        /// If IA32_EFER.NXE = 1, execute-disable (if 1, instruction fetches are not allowed from the 4-KByte page controlled by this entry; see Section 4.6); otherwise, reserved (must be 0)
        const XD = 1 << 63;
    }
}

/// Struct for invalidating the TLB (Translation Lookaside Buffer) entry.
///
/// This struct is responsible for invalidating a TLB entry associated with a
/// specific virtual address (`Va`). TLB entries are cached mappings between
/// virtual addresses and physical addresses, and they need to be invalidated
/// when the corresponding page table entries are modified or removed.
///
/// This struct provides methods for invalidating the TLB entry
/// and safely forgetting the modification. This internally holds the page
/// to be invalidated, to delay the free until the tlb entry is invalidated.
pub struct StaleTLBEntry(Va, Page);

impl core::ops::Deref for StaleTLBEntry {
    type Target = Page;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl core::ops::DerefMut for StaleTLBEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.1
    }
}

impl StaleTLBEntry {
    /// Create a new StaleTLBEntry.
    pub fn new(va: Va, page: Page) -> Self {
        Self(va, page)
    }

    /// Invalidate the underlying virtual address.
    ///
    /// This method issues an assembly instruction to invalidate the TLB entry
    /// corresponding to the given virtual address. The invalidation ensures
    /// that any cached translations are cleared and that the system will use
    /// the updated page table entries for subsequent address lookups.
    pub fn invalidate(self) -> Page {
        let va = self.0;
        let page = unsafe { core::ptr::read(&core::mem::ManuallyDrop::new(self).1) };

        unsafe {
            core::arch::asm!(
                "invlpg [{0}]",
                in(reg) va.into_usize(),
                options(nostack)
            );
        }

        TlbIpi::send(Cr3::current(), Some(va));
        page
    }
}

impl Drop for StaleTLBEntry {
    fn drop(&mut self) {
        panic!(
            "TLB entry for {:?} is not invalidated. You must call `.invalidate()`.",
            self.0,
        );
    }
}

/// Shutdown the TLB.
///
/// This method issues an assembly instruction to invalidate all TLB
/// entries of the current CPU. The invalidation ensures that any cached
/// translations are cleared and that the system will use the updated
/// page table entries for subsequent address lookups.
pub fn tlb_shutdown(pgtbl: &PageTableRoot) {
    let pgtbl_pa = pgtbl.pa().into_usize();
    let curr_cr3 = Cr3::current();

    if pgtbl_pa == curr_cr3.into_usize() {
        unsafe {
            core::arch::asm! {
                "mov rax, cr3",
                "mov cr3, rax",
                out("rax") _,
                options(nostack)
            }
        }
    }

    TlbIpi::send(Cr3(pgtbl_pa as u64), None);
}

/// Page Table Mapping Error.
///
/// This enum represents errors that can occur when working with page table
/// mappings in the virtual memory system. It is used to indicate specific
/// issues that arise during memory address mapping operations, such as setting
/// up or updating page tables.
#[derive(Debug, PartialEq, Eq)]
pub enum PageTableMappingError {
    /// Unaligned address.
    ///
    /// This error is returned when an address provided for a page table entry
    /// is not properly aligned to the required page size. For example, the
    /// address might not be a multiple of 4KB (on x86_64 systems).
    Unaligned,

    /// Not exist.
    ///
    /// This error is returned when a requested page table entry does not exist
    /// or is invalid. For instance, it could occur when trying to access an
    /// entry that is not present or has not been mapped yet.
    NotExist,

    /// Duplicated mapping.
    ///
    /// This error is returned when an attempt is made to create a duplicate
    /// mapping for an address that already has an existing mapping.
    Duplicated,

    /// Invalid permission.
    ///
    /// This error is returned when an attempt is made to create a mapping with
    /// an invalid permission.
    InvalidPermission,
}

bitflags::bitflags! {
    /// Possible memory permissions for a page.
    ///
    /// This defines the various permissions that can be assigned
    /// to memory pages in a page table. Each permission is represented by a single bit,
    /// allowing for efficient bitwise operations to check or modify permissions.
    ///
    /// The [`Permission`] allows you to specify memory access permissions such as:
    /// - Whether a page is readable.
    /// - Whether a page is writable.
    /// - Whether a page is executable.
    /// - Whether a page can be accessed by user applications.
    pub struct Permission: usize {
        /// Page is readable.
        ///
        /// This permission allows read access to the page. The page can be
        /// accessed for reading data.
        const READ = 1 << 0;

        /// Page is writable.
        ///
        /// This permission allows write access to the page. The page can be
        /// modified by a process.
        const WRITE = 1 << 1;

        /// Page is executable.
        ///
        /// This permission allows the page to be executed. The page can contain
        /// code that is executed by the CPU, such as instructions.
        const EXECUTABLE = 1 << 2;

        /// Page can be referred by user application.
        ///
        /// This permission allows the page to be accessed by user-mode applications.
        /// Typically, the kernel uses this flag to differentiate between user-mode and
        /// kernel-mode access.
        const USER = 1 << 3;
    }
}

impl Permission {
    /// All possible permissions.
    pub const ALL_CASES: [Permission; 16] = [
        Permission::from_bits_truncate(0),
        Permission::from_bits_truncate(1),
        Permission::from_bits_truncate(2),
        Permission::from_bits_truncate(3),
        Permission::from_bits_truncate(4),
        Permission::from_bits_truncate(5),
        Permission::from_bits_truncate(6),
        Permission::from_bits_truncate(7),
        Permission::from_bits_truncate(8),
        Permission::from_bits_truncate(9),
        Permission::from_bits_truncate(10),
        Permission::from_bits_truncate(11),
        Permission::from_bits_truncate(12),
        Permission::from_bits_truncate(13),
        Permission::from_bits_truncate(14),
        Permission::from_bits_truncate(15),
    ];
}

/// A page table root.
///
/// It wraps the Pml4e array to ensure page table align to 4096.
/// Note that it is not allowed to modified the larger indices than
/// [`Self::KBASE`], which are reserved for kernel address.
#[repr(align(4096))]
#[derive(Debug)]
pub struct PageTableRoot([Pml4e; 512]);

impl Deref for PageTableRoot {
    type Target = [Pml4e; 512];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::Index<usize> for PageTableRoot {
    type Output = Pml4e;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl core::ops::IndexMut<usize> for PageTableRoot {
    // Required method
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= Self::KBASE {
            let kernel_pt = unsafe {
                (Pa::new({
                    unsafe extern "C" {
                        static mut boot_pml4e: u64;
                    }
                    boot_pml4e as usize
                })
                .unwrap()
                .into_kva()
                .into_usize() as *const [Pml4e; 512])
                    .as_ref()
                    .unwrap()
            };
            if kernel_pt[index].pa().is_some() && kernel_pt[index].pa() == self.0[index].pa() {
                panic!(
                    "Trying to modify entries for kernel page table: {} (limit: {}).",
                    index,
                    Self::KBASE
                );
            }
        }
        &mut self.0[index]
    }
}

impl PageTableRoot {
    /// Base of pml4 index occupied for kernel address.
    pub const KBASE: usize = 256;

    /// Create a empty [`PageTableRoot`].
    pub fn new_boxed() -> Box<Self> {
        Box::new(PageTableRoot([Pml4e(0); 512]))
    }

    /// Create a new [`PageTableRoot`] that allowed to access the kernel
    /// addresses.
    pub fn new_boxed_with_kernel_addr() -> Box<Self> {
        let kernel_pt = unsafe {
            (Pa::new({
                unsafe extern "C" {
                    static mut boot_pml4e: u64;
                }
                boot_pml4e as usize
            })
            .unwrap()
            .into_kva()
            .into_usize() as *const [Pml4e; 512])
                .as_ref()
                .unwrap()
        };
        let mut this = Self::new_boxed();
        this.0[Self::KBASE..512].copy_from_slice(&kernel_pt[Self::KBASE..512]);
        this
    }

    /// Get the physical address of this page table root.
    pub fn pa(&self) -> Pa {
        crate::mm::Kva::new(self.as_ptr() as usize)
            .unwrap()
            .into_pa()
    }
}

#[doc(hidden)]
pub(crate) static ACTIVE_PAGE_TABLES: [AtomicUsize; MAX_CPU] =
    [const { AtomicUsize::new(0) }; MAX_CPU];

/// Load page table by given physical address.
#[inline]
pub fn load_pt(pa: Pa) {
    if abyss::x86_64::Cr3::current().into_usize() != pa.into_usize() {
        // println!("RELOAD PT {:?}", pa);
        ACTIVE_PAGE_TABLES[abyss::x86_64::intrinsics::cpuid()].store(pa.into_usize());
        unsafe { abyss::x86_64::Cr3(pa.into_usize() as u64).apply() }
    }
}

/// Get current page table's physical address.
#[inline]
pub fn get_current_pt_pa() -> Pa {
    let addr = abyss::x86_64::Cr3::current().into_usize();
    assert_eq!(
        ACTIVE_PAGE_TABLES[abyss::x86_64::intrinsics::cpuid()].load(),
        addr
    );
    Pa::new(addr).unwrap()
}
