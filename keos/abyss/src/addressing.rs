//! Physical and Virtual Memory Addressing Schemes.
//!
//! This module provides abstractions for virtual address and physical
//! address. In the keos kernel, kernel virtual memory is directly mapped to
//! physical memory. The first page of kernel virtual memory maps to the first
//! frame of physical memory, the second page maps to the second frame, and so
//! on. This direct mapping allows the kernel to calculate the physical address
//! from the kernel virtual address with simple arithmetic operations, by adding
//! or subtracting a constant offset.
//!
//! The module defines three primary types for memory addresses: [`Kva`] for
//! kernel virtual address, [`Va`] for virtual address, and [`Pa`] for
//! physical address. These types are equipped with methods to facilitate
//! address manipulation, conversion between virtual and physical addresses, and
//! safe arithmetic operations on addresses.
//!
//! In this abstraction, the kernel can seamlessly perform address calculations
//! and fastly convert between physical and kernel virtual adresses.
//!
//! Both [`Pa`], [`Va`] and [`Kva`] support arithmetic operations (addition,
//! subtraction, bitwise operations), which allow straightforward address
//! arithmetic. The types can be safely cast between virtual and physical
//! addresses, and the operations maintain the integrity of the memory model.
//!
//! ## Arithmetic Operations
//!
//! Both `Pa` and `Kva` types implement various arithmetic operations such as
//! addition, subtraction, and bitwise operations. These operations allow for
//! easy manipulation of addresses, such as incrementing or decrementing by
//! a specific number of bytes or performing logical operations on addresses.
//!
//! ## Example Usage:
//!
//! ```
//! // Create a physical address
//! let pa = Pa::new(0x1234_5678_9abc_0000).unwrap();
//!
//! // Convert to kernel virtual address
//! let kva = pa.into_kva();
//!
//! // Perform arithmetic on addresses
//! let next_pa = pa + 0x1000; // Move to the next page
//! ```

const VA_TO_PA_OFF: usize = 0xffff000000000000 | (510 << 39);

/// The size of a single page in memory, in bytes.
///
/// This constant represents the size of a memory page, which is 4 KiB
/// (kilobytes). It is commonly used in memory management to divide memory into
/// pages for efficient allocation, paging, and address translation.
///
/// This value is crucial when working with memory in the kernel, as it
/// determines how memory is accessed, mapped, and managed. It is used in
/// conjunction with other constants (e.g., page shift, page mask)
/// to handle operations like page table indexing, virtual-to-physical address
/// translation, and memory page allocation.
///
/// ## Example:
/// ```
/// let next_page = address + PAGE_SIZE;
/// ```
pub const PAGE_SIZE: usize = 0x1000;

/// The shift amount to get the page index from a given address.
///
/// This constant represents the number of bits to shift a memory address to
/// obtain the page index. It is used to determine which page a given address
/// belongs to by shifting the address to the right. This value corresponds to
/// the log2 of the page size (which is typically 12 bits for a 4 KB page size).
///
/// ## Example:
/// ```
/// let frame_number = address >> PAGE_SHIFT;
/// ```
pub const PAGE_SHIFT: usize = 12; // 12 bits (log2 of 4 KB)

/// A mask for extracting the offset within a page from a given address.
///
/// This constant is used to calculate the offset within a page from a given
/// address. By using the page size and the corresponding shift value, this mask
/// allows you to calculate the exact byte offset within the page.
/// It is commonly used in address translation and memory management to
/// determine the location of a byte relative to the start of the page.
///
/// ## Example:
/// ```
/// let offset_within_page = address & PAGE_MASK;  // Get the byte offset within the page
/// ```
pub const PAGE_MASK: usize = 0xfff;

/// Represents a physical address.
///
/// The `Pa` (Physical Address) struct is a wrapper around the `usize` type,
/// which represents a physical address in memory. It is used to handle
/// addresses that correspond directly to the hardware memory locations in the
/// physical address space.
///
/// This struct provides methods to:
/// - Create a new physical address with validation.
/// - Convert a physical address to a virtual address.
/// - Convert a physical address to a raw `usize` type.
///
/// ## Example:
/// ```
/// let pa = Pa::new(0x1234_5678_9ABC_DEF0).unwrap();
/// let kva = pa.into_kva(); // Convert to a kernel virtual address.
/// ```
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct Pa(usize);

impl Pa {
    /// The physical address `0`.
    ///
    /// This constant represents the special address `0`.
    pub const ZERO: Self = Self(0);

    /// Creates a new physical address if the address is valid.
    ///
    /// This method attempts to create a new [`Pa`] instance by validating the
    /// provided physical address. The address must be less than
    /// `0xffff_0000_0000_0000`, which ensures it falls within the valid
    /// physical address range.
    ///
    /// # Arguments
    /// - `addr`: A `usize` representing the physical address.
    ///
    /// # Returns
    /// - `Some(Pa)` if the address is valid.
    /// - `None` if the address is outside the valid range.
    ///
    /// ## Example:
    /// ```rust
    /// let pa = Pa::new(0x1234_5678_9ABC_DEF0);
    /// ```
    #[inline]
    pub const fn new(addr: usize) -> Option<Self> {
        if addr < 0xffff_0000_0000_0000 {
            Some(Self(addr))
        } else {
            None
        }
    }

    /// Cast the physical address into a raw `usize`.
    ///
    /// This method allows the physical address to be cast into a raw `usize`
    /// value, which can be used for low-level operations like pointer
    /// arithmetic or addressing.
    ///
    /// # Returns
    /// - The underlying `usize` value representing the physical address.
    #[inline]
    pub const fn into_usize(self) -> usize {
        self.0
    }

    /// Convert the physical address to a virtual address.
    ///
    /// This method allows you to convert a [`Pa`] (physical address) to a
    /// [`Kva`] (kernel virtual address). The conversion uses a fixed offset
    /// to transform the physical address into a corresponding kernel virtual
    /// address.
    ///
    /// # Returns
    /// - The corresponding kernel virtual address as a [`Kva`] instance.
    ///
    /// ## Example:
    /// ```rust
    /// let pa = Pa::new(0x1234_5678_9ABC_DEF0).unwrap();
    /// let va = pa.into_kva();  // Convert the physical address to kernel virtual address
    /// ```
    #[inline]
    pub const fn into_kva(self) -> Kva {
        Kva(self.0 + VA_TO_PA_OFF)
    }

    /// Align down the physical address to the page boundary.
    pub const fn page_down(self) -> Self {
        Self(self.0 & !PAGE_MASK)
    }

    /// Align up to the physical address to the page boundary.
    pub const fn page_up(self) -> Self {
        Self((self.0 + PAGE_MASK) & !PAGE_MASK)
    }

    /// Extracts the page offset from the physical address.
    ///
    /// This method retrieves the lower bits of the address that represent the
    /// offset within a memory page. The offset is useful when working with
    /// memory operations that need to determine the position within a page.
    ///
    /// # Returns
    /// - The offset within the page as a `usize`.
    ///
    /// # Example
    /// ```
    /// let pa = Pa::new(0x1234_5678).unwrap();
    /// let offset = pa.offset();
    /// assert_eq!(offset, 0x678); // Example offset within the page
    /// ```
    #[inline]
    pub const fn offset(self) -> usize {
        self.0 & PAGE_MASK
    }
}

/// Represents a kernel virtual address.
///
/// The [`Kva`] (Kernel Virtual Address) struct is a lightweight wrapper around
/// a `usize` value that represents an address in the kernel's virtual address
/// space. It provides utility methods for address validation, conversion, and
/// alignment.
///
/// This abstraction ensures that kernel addresses are used safely and
/// consistently, reducing the risk of incorrect memory access.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct Kva(usize);

impl Kva {
    /// Creates a new kernel virtual address if the address is valid.
    ///
    /// This method validates the given address to ensure it falls within the
    /// valid kernel virtual address range. If the address is valid, it
    /// returns a `Some(Kva)`, otherwise, it returns `None`.
    ///
    /// # Arguments
    /// - `addr`: A `usize` representing the virtual address.
    ///
    /// # Returns
    /// - `Some(Kva)` if the address is within the valid kernel address space.
    /// - `None` if the address is outside the valid range.
    ///
    /// # Example
    /// ```
    /// let kva = Kva::new(0xFFFF_8000_1234_5678);
    /// assert!(kva.is_some()); // Valid kernel virtual address
    ///
    /// let invalid_kva = Kva::new(0x1234_5678);
    /// assert!(invalid_kva.is_none()); // Invalid kernel address
    /// ```
    #[inline(always)]
    pub const fn new(addr: usize) -> Option<Self> {
        match addr & 0xffff_8000_0000_0000 {
            0xffff_8000_0000_0000 => Some(Self(addr)),
            _ => None,
        }
    }

    /// Returns the raw `usize` representation of the virtual address.
    ///
    /// This function extracts the underlying `usize` value from the [`Kva`]
    /// struct, allowing it to be used in low-level operations.
    ///
    /// # Returns
    /// - The virtual address as a `usize` value.
    ///
    /// # Example
    /// ```
    /// let kva = Kva::new(0xFFFF_8000_1234_5678).unwrap();
    /// let raw_addr = kva.into_usize();
    /// assert_eq!(raw_addr, 0xFFFF_8000_1234_5678);
    /// ```
    #[inline]
    pub const fn into_usize(self) -> usize {
        self.0
    }

    /// Converts the virtual address to a physical address.
    ///
    /// This method maps a [`Kva`] (Kernel Virtual Address) to a [`Pa`]
    /// (Physical Address) by subtracting a fixed offset. This operation
    /// utilizes that the virtual address follows a known mapping pattern
    /// for kernel memory.
    ///
    /// # Returns
    /// - The corresponding physical address as a [`Pa`] instance.
    ///
    /// # Example
    /// ```
    /// let kva = Kva::new(0xFFFF_8000_1234_5678).unwrap();
    /// let pa = kva.into_pa();
    /// ```
    #[inline]
    pub const fn into_pa(self) -> Pa {
        Pa(self.0 - VA_TO_PA_OFF)
    }

    /// Aligns the virtual address down to the nearest page boundary.
    ///
    /// This method clears the lower bits of the address to ensure it is
    /// page-aligned downwards, meaning the address will be rounded down to
    /// the start of the current memory page.
    ///
    /// # Returns
    /// - A new [`Kva`] instance representing the aligned address.
    ///
    /// # Example
    /// ```
    /// let kva = Kva::new(0xFFFF_8000_1234_5678).unwrap();
    /// let aligned = kva.page_down();
    /// assert_eq!(aligned.into_usize(), 0xFFFF_8000_1234_5000); // Example of alignment
    /// ```
    #[inline]
    pub const fn page_down(self) -> Self {
        Self(self.0 & !PAGE_MASK)
    }

    /// Aligns the virtual address up to the nearest page boundary.
    ///
    /// This method rounds the address up to the next page boundary by adding
    /// [`PAGE_MASK`] and clearing the lower bits. This ensures that the
    /// address is aligned to the start of the next memory page.
    ///
    /// # Returns
    /// - A new [`Kva`] instance representing the aligned address.
    ///
    /// # Example
    /// ```
    /// let kva = Kva::new(0xFFFF_8000_1234_5678).unwrap();
    /// let aligned = kva.page_up();
    /// assert_eq!(aligned.into_usize(), 0xFFFF_8000_1234_6000); // Example of alignment
    /// ```
    #[inline]
    pub const fn page_up(self) -> Self {
        Self((self.0 + PAGE_MASK) & !PAGE_MASK)
    }

    /// Extracts the page offset from the virtual address.
    ///
    /// This method retrieves the lower bits of the address that represent the
    /// offset within a memory page. The offset is useful when working with
    /// memory operations that need to determine the position within a page.
    ///
    /// # Returns
    /// - The offset within the page as a `usize`.
    ///
    /// # Example
    /// ```
    /// let kva = Kva::new(0xFFFF_8000_1234_5678).unwrap();
    /// let offset = kva.offset();
    /// assert_eq!(offset, 0x678); // Example offset within the page
    /// ```
    #[inline]
    pub const fn offset(self) -> usize {
        self.0 & PAGE_MASK
    }

    /// Converts the kernel virtual address into a general virtual address.
    ///
    /// This method allows converting a [`Kva`] into a [`Va`] (a more generic
    /// virtual address type), maintaining the same underlying address value
    /// but changing its type representation.
    ///
    /// # Returns
    /// - The equivalent [`Va`] instance.
    ///
    /// # Example
    /// ```
    /// let kva = Kva::new(0xFFFF_8000_1234_5678).unwrap();
    /// let va = kva.into_va();
    /// ```
    #[inline]
    pub const fn into_va(self) -> Va {
        Va(self.0)
    }
}

/// Represents a virtual address.
///
/// The [`Va`] (Virtual Address) struct represents an address in the virtual
/// memory space used by the kernel or user-space applications.
///
/// This abstraction provides utility methods for validation, alignment, and
/// address manipulation, ensuring safe and consistent handling of virtual
/// addresses.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct Va(usize);

impl Va {
    /// Creates a new virtual address if the address is valid.
    ///
    /// This method checks whether the given address falls within the valid
    /// virtual address range. If it does, a `Some(Va)` is returned;
    /// otherwise, `None` is returned.
    ///
    /// # Arguments
    /// - `addr`: A `usize` representing the virtual address.
    ///
    /// # Returns
    /// - `Some(Va)`: If the address is within the valid virtual memory range.
    /// - `None`: If the address is invalid.
    ///
    /// # Example
    /// ```
    /// let va = Va::new(0xFFFF_8000_1234_5678);
    /// assert!(va.is_some()); // Valid virtual address
    ///
    /// let invalid_va = Va::new(0xFFFF_7000_1234_5678);
    /// assert!(invalid_va.is_none()); // Invalid virtual address
    /// ```
    #[inline(always)]
    pub const fn new(addr: usize) -> Option<Self> {
        match addr & 0xffff_8000_0000_0000 {
            m if m == 0xffff_8000_0000_0000 || m == 0 => Some(Self(addr)),
            _ => None,
        }
    }

    /// Returns the raw `usize` representation of the virtual address.
    ///
    /// This method allows extracting the underlying `usize` value, enabling
    /// low-level operations or conversions between address types.
    ///
    /// # Returns
    /// - The virtual address as a `usize`.
    ///
    /// # Example
    /// ```
    /// let va = Va::new(0xFFFF_8000_1234_5678).unwrap();
    /// let raw_addr = va.into_usize();
    /// assert_eq!(raw_addr, 0xFFFF_8000_1234_5678);
    /// ```
    #[inline]
    pub const fn into_usize(self) -> usize {
        self.0
    }

    /// Aligns the virtual address down to the nearest page boundary.
    ///
    /// This method ensures that the address is aligned to the start of its
    /// memory page by clearing the lower bits that represent the page
    /// offset.
    ///
    /// # Returns
    /// - A new [`Va`] instance representing the aligned address.
    ///
    /// # Example
    /// ```
    /// let va = Va::new(0xFFFF_8000_1234_5678).unwrap();
    /// let aligned = va.page_down();
    /// assert_eq!(aligned.into_usize(), 0xFFFF_8000_1234_5000); // Example alignment
    /// ```
    #[inline]
    pub const fn page_down(self) -> Self {
        Self(self.0 & !PAGE_MASK)
    }

    /// Aligns the virtual address up to the nearest page boundary.
    ///
    /// This method rounds up the address to the next memory page.
    ///
    /// # Returns
    /// - A new [`Va`] instance representing the aligned address.
    ///
    /// # Example
    /// ```
    /// let va = Va::new(0xFFFF_8000_1234_5678).unwrap();
    /// let aligned = va.page_up();
    /// assert_eq!(aligned.into_usize(), 0xFFFF_8000_1234_6000); // Example alignment
    /// ```
    #[inline]
    pub const fn page_up(self) -> Self {
        Self((self.0 + PAGE_MASK) & !PAGE_MASK)
    }

    /// Extracts the offset within the memory page from the virtual address.
    ///
    /// This method retrieves the lower bits of the address that indicate the
    /// position within a page, which is useful for memory management and
    /// page-related operations.
    ///
    /// # Returns
    /// - The offset within the page as a `usize`.
    ///
    /// # Example
    /// ```
    /// let va = Va::new(0xFFFF_8000_1234_5678).unwrap();
    /// let offset = va.offset();
    /// assert_eq!(offset, 0x678); // Example offset within the page
    /// ```
    #[inline]
    pub const fn offset(self) -> usize {
        self.0 & PAGE_MASK
    }
}

macro_rules! impl_arith {
    ($t: ty) => {
        impl core::ops::Add<usize> for $t {
            type Output = Self;

            fn add(self, other: usize) -> Self::Output {
                Self(self.0 + other)
            }
        }
        impl core::ops::AddAssign<usize> for $t {
            fn add_assign(&mut self, other: usize) {
                self.0 = self.0 + other
            }
        }
        impl core::ops::Sub<usize> for $t {
            type Output = Self;

            fn sub(self, other: usize) -> Self::Output {
                Self(self.0 - other)
            }
        }
        impl core::ops::Sub<Self> for $t {
            type Output = usize;

            fn sub(self, other: Self) -> Self::Output {
                self.0 - other.0
            }
        }
        impl core::ops::SubAssign<usize> for $t {
            fn sub_assign(&mut self, other: usize) {
                self.0 = self.0 - other
            }
        }
        impl core::ops::BitOr<usize> for $t {
            type Output = Self;

            fn bitor(self, other: usize) -> Self {
                Self(self.0 | other)
            }
        }
        impl core::ops::BitOrAssign<usize> for $t {
            fn bitor_assign(&mut self, other: usize) {
                self.0 = self.0 | other;
            }
        }
        impl core::ops::BitAnd<usize> for $t {
            type Output = Self;

            fn bitand(self, other: usize) -> Self {
                Self(self.0 & other)
            }
        }
        impl core::ops::BitAndAssign<usize> for $t {
            fn bitand_assign(&mut self, other: usize) {
                self.0 = self.0 & other;
            }
        }
    };
}

impl_arith!(Kva);
impl_arith!(Va);
impl_arith!(Pa);

impl core::fmt::Debug for Kva {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Kva(0x{:x})", self.0)
    }
}
impl core::fmt::Display for Kva {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Kva(0x{:x})", self.0)
    }
}
impl core::fmt::Debug for Va {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Va(0x{:x})", self.0)
    }
}
impl core::fmt::Display for Va {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Va(0x{:x})", self.0)
    }
}
impl core::fmt::Debug for Pa {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Pa(0x{:x})", self.0)
    }
}
impl core::fmt::Display for Pa {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Pa(0x{:x})", self.0)
    }
}
