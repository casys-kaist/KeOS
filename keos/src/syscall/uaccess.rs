//! The `uaccess` module provides abstractions for interacting with user-space
//! memory in a kernel context.
//!
//! This module defines several types of user-space pointers that allow the
//! kernel to access user-space data with various access modes, such as
//! read-only, write-only, or read-write.
//!
//! The types provided by this module include:
//!
//! - [`UserPtrRO`]: A one-time, read-only pointer to a user-space object of
//!   type `T`. It allows the kernel to read from user-space but does not permit
//!   writing to the data.
//! - [`UserPtrWO`]: A one-time, write-only pointer to a user-space object of
//!   type `T`. It allows the kernel to write data to user-space but does not
//!   permit reading from it.
//! - [`UserU8SliceRO`]: A one-time, read-only pointer to a slice of `u8` in
//!   user-space. This type allows the kernel to read byte slices from
//!   user-space.
//! - [`UserU8SliceWO`]: A one-time, write-only pointer to a slice of `u8` in
//!   user-space. This type allows the kernel to write byte slices to
//!   user-space.
//! - [`UserCString`]: A utility to handle C-style null-terminated strings from
//!   user-space. It provides methods for reading and converting the string into
//!   a `String` in the kernel.
//!
//! These types use unsafe code to access memory directly. The user-space
//! addresses must be valid and within bounds to prevent undefined behavior or
//! security vulnerabilities. To ensure the memory safety, these types use
//! [`Task::access_ok`] before accessing user-space memory. This function
//! verifies that the provided memory range is valid and accessible, preventing
//! potential security vulnerabilities and undefined behavior. If the memory is
//! not accessible, the operation will fail gracefully instead of causing
//! undefined behavior.
use crate::KernelError;
#[cfg(doc)]
use crate::task::Task;
use crate::thread::with_current;
use abyss::addressing::Va;
use alloc::string::String;
use alloc::vec::Vec;

/// A one-time, read-only pointer to a user-space object of type `T`.
///
/// This struct allows the kernel to read from user-space while ensuring
/// safe access patterns. It prevents TOCTOU (Time-of-Check to Time-of-Use)
/// attacks by taking ownership of the pointer during operations.
///
/// # Type Parameter
/// - `T`: The type of the data being accessed. Must implement `Copy`.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct UserPtrRO<T>
where
    T: Copy,
{
    addr: usize,
    _ty: core::marker::PhantomData<T>,
}

impl<T> UserPtrRO<T>
where
    T: Copy,
{
    /// Creates a new `UserPtrRO` instance with the given user-space address.
    pub fn new(addr: usize) -> Self {
        UserPtrRO {
            addr,
            _ty: core::marker::PhantomData,
        }
    }

    /// Reads a value of type `T` from the user-space address.
    ///
    /// Takes ownership of `self` to prevent TOCTOU attacks.
    ///
    /// Returns `Ok(T)` if successful, otherwise
    /// `Err(KernelError::BadAddress)`.
    pub fn get(self) -> Result<T, KernelError> {
        let access_range = Va::new(self.addr).ok_or(KernelError::BadAddress)?
            ..Va::new(self.addr + core::mem::size_of::<T>()).ok_or(KernelError::BadAddress)?;
        with_current(|th| {
            let task = th
                .task
                .as_ref()
                .expect("Try to call UserPtrRO::get() on the kernel thread.");
            if task.access_ok(access_range, false) {
                Ok(unsafe { { self.addr as *const T }.read_unaligned() })
            } else {
                Err(KernelError::BadAddress)
            }
        })
    }
}

/// A one-time, write-only pointer to a user-space object of type `T`.
///
/// This struct allows the kernel to write to user-space while ensuring
/// safe access patterns. It prevents TOCTOU (Time-of-Check to Time-of-Use)
/// attacks by taking ownership of the pointer during operations.
///
/// # Type Parameter
/// - `T`: The type of the data being accessed. Must implement `Copy`.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct UserPtrWO<T>
where
    T: Copy,
{
    addr: usize,
    _ty: core::marker::PhantomData<T>,
}

impl<T> UserPtrWO<T>
where
    T: Copy,
{
    /// Creates a new `UserPtrWO` instance with the given user-space address.
    pub fn new(addr: usize) -> Self {
        UserPtrWO {
            addr,
            _ty: core::marker::PhantomData,
        }
    }

    /// Writes a value of type `T` to the user-space address.
    ///
    /// Takes ownership of `self` to prevent TOCTOU attacks.
    ///
    /// Returns `Ok(usize)` indicating the number of bytes written, or
    /// `Err(KernelError::BadAddress)` on failure.
    pub fn put(self, other: T) -> Result<usize, KernelError> {
        let access_range = Va::new(self.addr).ok_or(KernelError::BadAddress)?
            ..Va::new(self.addr + core::mem::size_of::<T>()).ok_or(KernelError::BadAddress)?;
        with_current(|th| {
            let task = th
                .task
                .as_ref()
                .expect("Try to call UserPtrWO::put() on the kernel thread.");
            if task.access_ok(access_range, true) {
                let target = self.addr as *mut T;
                unsafe {
                    // Safety: By calling access_ok, verifying `target` is valid, aligned, and
                    // accessible.
                    *target = other;
                }
                Ok(core::mem::size_of::<T>())
            } else {
                Err(KernelError::BadAddress)
            }
        })
    }
}

/// A one-time, read-only pointer to a slice of `u8` in user-space.
///
/// This struct allows the kernel to safely read from a user-space buffer while
/// preventing TOCTOU attacks by taking ownership of the pointer during
/// operations.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct UserU8SliceRO {
    addr: usize,
    len: usize,
}

impl UserU8SliceRO {
    /// Creates a new `UserU8SliceRO` instance with the given user-space address
    /// and length.
    pub fn new(addr: usize, len: usize) -> Self {
        UserU8SliceRO { addr, len }
    }

    /// Reads data from the user-space buffer into a `Vec<u8>`.
    ///
    /// Takes ownership of `self` to prevent TOCTOU attacks.
    ///
    /// Returns `Ok(Vec<u8>)` containing the data if successful, otherwise
    /// `Err(KernelError::BadAddress)`.
    pub fn get(self) -> Result<Vec<u8>, KernelError> {
        let access_range = Va::new(self.addr).ok_or(KernelError::BadAddress)?
            ..Va::new(self.addr + self.len).ok_or(KernelError::BadAddress)?;
        with_current(|th| {
            let task = th
                .task
                .as_ref()
                .expect("Try to call UserU8SliceRO::get() on the kernel thread.");
            if task.access_ok(access_range, false) {
                let mut result = Vec::new();
                result.extend_from_slice(unsafe {
                    core::slice::from_raw_parts(self.addr as *const u8, self.len)
                });
                Ok(result)
            } else {
                Err(KernelError::BadAddress)
            }
        })
    }
}

/// A one-time, write-only pointer to a slice of `u8` in user-space.
///
/// This struct allows the kernel to safely write to a user-space buffer while
/// preventing TOCTOU attacks by taking ownership of the pointer during
/// operations.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct UserU8SliceWO {
    addr: usize,
    len: usize,
}

impl UserU8SliceWO {
    /// Creates a new `UserU8SliceWO` instance with the given user-space address
    /// and length.
    pub fn new(addr: usize, len: usize) -> Self {
        UserU8SliceWO { addr, len }
    }
    /// Writes data from a slice to the user-space buffer.
    ///
    /// Takes ownership of `self` to prevent TOCTOU attacks.
    ///
    /// Returns `Ok(usize)` indicating the number of bytes written, or
    /// `Err(KernelError::BadAddress)` on failure.
    pub fn put(self, other: &[u8]) -> Result<usize, KernelError> {
        let size = self.len.min(other.len());
        let access_range = Va::new(self.addr).ok_or(KernelError::BadAddress)?
            ..Va::new(self.addr + self.len).ok_or(KernelError::BadAddress)?;
        with_current(|th| {
            let task = th
                .task
                .as_ref()
                .expect("Try to call UserU8SliceWO::put() on the kernel thread.");
            if task.access_ok(access_range, true) {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        other[..size].as_ptr(),
                        self.addr as *mut u8,
                        size,
                    );
                }
                Ok(size)
            } else {
                Err(KernelError::BadAddress)
            }
        })
    }
}

/// A pointer to a null-terminated C-style string in user-space.
///
/// This struct provides a safe abstraction for reading strings from user-space.
/// It iterates over the bytes until a null-terminator (`0x00`) is encountered,
/// converting the byte sequence into a valid UTF-8 `String`.
#[derive(PartialEq, PartialOrd, Eq, Ord, Debug)]
pub struct UserCString {
    addr: usize,
}

impl UserCString {
    /// Creates a new `UserCString` instance with the given user-space address.
    pub fn new(addr: usize) -> Self {
        Self { addr }
    }

    /// Reads a null-terminated string from the user-space address.
    ///
    /// This function iterates over user-space memory, collecting bytes until
    /// a null terminator (`0x00`) is found. It then attempts to convert the
    /// byte sequence into a UTF-8 `String`.
    ///
    /// Returns `Some(String)` if successful, otherwise `None` if the operation
    /// fails.
    pub fn read(self) -> Result<String, KernelError> {
        let mut ptr = self.addr;
        let mut result = Vec::new();
        // Iterate over the bytes to find the null-terminator (0x00).
        // If the byte is 0, we've found the null-terminator.
        loop {
            match UserPtrRO::<u8>::new(ptr).get() {
                Ok(0) => {
                    return core::str::from_utf8(&result)
                        .ok()
                        .map(String::from)
                        .ok_or(KernelError::InvalidArgument);
                }
                Ok(v) => {
                    ptr += 1;
                    result.push(v);
                }
                Err(e) => return Err(e),
            }
        }
    }
}
