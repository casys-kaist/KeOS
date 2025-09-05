// Copyright 2025 Computer Architecture and Systems Lab
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Mmio interface.

use crate::addressing::{Kva, Pa};

/// Type for accessing mmio register.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct MmioAccessor<T, const R: bool, const W: bool>(pub *mut T);

unsafe impl<T, const R: bool, const W: bool> Send for MmioAccessor<T, R, W> {}

impl<T, const W: bool> MmioAccessor<T, true, W> {
    /// Read from the register.
    ///
    /// # Safety
    /// Mmio region must be mapped to the valid virtual address.
    #[inline(always)]
    pub fn read(&self) -> T {
        unsafe { core::ptr::read_volatile(self.0) }
    }
}

impl<T, const R: bool> MmioAccessor<T, R, true> {
    /// Write to the register.
    ///
    /// # Safety
    /// Mmio region must be mapped to the valid virtual address.
    #[inline(always)]
    pub fn write(&self, v: T) {
        unsafe { core::ptr::write_volatile::<T>(self.0, v) }
    }
}

/// Type for accessing array of mmio registers.
#[derive(Clone, Copy, Debug)]
pub struct MmioArrayAccessor<T, const R: bool, const W: bool, const SZ: usize>(
    pub *mut T,
    pub usize,
);

impl<T, const W: bool, const SZ: usize> MmioArrayAccessor<T, true, W, SZ> {
    /// Read from the register.
    ///
    /// # Safety
    /// Mmio region must be mapped to the valid virtual address.
    #[inline(always)]
    pub fn read_at(&self, idx: usize) -> T {
        unsafe {
            core::assert!(idx < SZ);
            core::ptr::read_volatile((self.0 as usize + idx * self.1) as *mut T)
        }
    }
}

impl<T, const R: bool, const SZ: usize> MmioArrayAccessor<T, R, true, SZ> {
    /// Write to the register.
    ///
    /// # Safety
    /// Mmio region must be mapped to the valid virtual address.
    #[inline(always)]
    pub fn write_at(&self, idx: usize, v: T) {
        core::assert!(idx < SZ);
        unsafe { core::ptr::write_volatile::<T>((self.0 as usize + idx * self.1) as *mut T, v) }
    }
}

/// Representation of Mmio area.
#[repr(transparent)]
#[derive(Debug)]
pub struct MmioArea(pub core::ops::Range<Pa>);

impl MmioArea {
    /// Create a new mmio area.
    ///
    /// # Safety
    /// mmio range should be valid.
    #[inline(always)]
    pub const unsafe fn new(n: core::ops::Range<Pa>) -> Self {
        Self(n)
    }

    /// Get size of this mmio area.
    #[inline(always)]
    pub const fn size(&self) -> usize {
        self.0.end.into_usize() - self.0.start.into_usize()
    }

    /// Activate this mmio area.
    #[inline(always)]
    pub fn activate(self) -> ActiveMmioArea {
        let core::ops::Range { start, end } = self.0;

        ActiveMmioArea(start.into_kva()..end.into_kva())
    }

    /// Clone this mmio area.
    ///
    /// # Safety
    /// The caller must synchronize the access of the duplicated mmio area.
    pub unsafe fn clone(&self) -> Self {
        Self(self.0.clone())
    }

    /// Divides one mmio area into two at an index.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding
    /// the index `mid` itself) and the second will contain all
    /// indices from `[mid, len)` (excluding the index `len` itself).
    ///
    /// # Panics
    ///
    /// Panics if `mid > len`.
    pub fn split_at(self, mid: usize) -> (Self, Self) {
        assert!(mid <= self.size());
        let core::ops::Range { start, end } = self.0;
        let mid = start + mid;
        (Self(start..mid), Self(mid..end))
    }
}

/// Represent the activated mmio area.
#[repr(transparent)]
#[derive(Debug)]
pub struct ActiveMmioArea(core::ops::Range<Kva>);

impl ActiveMmioArea {
    /// Return range of this mmio area as virtual.
    #[inline]
    pub fn start_end(&self) -> (usize, usize) {
        (self.0.start.into_usize(), self.0.end.into_usize())
    }

    /// Write value at `of`.
    pub fn write_at<T: Copy>(&self, of: usize, t: T) {
        unsafe {
            assert!(
                self.0.start.into_usize() + of * core::mem::size_of::<T>()
                    < self.0.end.into_usize()
            );
            core::ptr::write_volatile(
                ((self.0.start.into_usize() + of * core::mem::size_of::<T>()) as *mut T)
                    .as_mut()
                    .unwrap(),
                t,
            );
        }
    }

    /// Read value from `of`.
    pub fn read_at<T: Copy>(&self, of: usize) -> T {
        unsafe {
            assert!(
                self.0.start.into_usize() + of * core::mem::size_of::<T>()
                    < self.0.end.into_usize()
            );
            core::ptr::read_volatile(
                ((self.0.start.into_usize() + of * core::mem::size_of::<T>()) as *mut T)
                    .as_ref()
                    .unwrap(),
            )
        }
    }
}

#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! __mmio_mk_register {
    ($e:ident, $(#[$attr:meta])* $N:ident @ $off:expr_2021 => R, $T:ty; $($t:tt)*) => {
        __mmio_mk_register!(@MAKE, $e, $(#[$attr])*, $N, $T, $off, true, false);
        __mmio_mk_register!($e, $($t)*);
    };
    ($e:ident, $(#[$attr:meta])* $N:ident @ $off:expr_2021 => W, $T:ty; $($t:tt)*) => {
        __mmio_mk_register!(@MAKE, $e, $(#[$attr])*, $N, $T, $off, false, true);
        __mmio_mk_register!($e, $($t)*);
    };
    ($e:ident, $(#[$attr:meta])* $N:ident @ $off:expr_2021 => RW, $T:ty; $($t:tt)*) => {
        __mmio_mk_register!(@MAKE, $e, $(#[$attr])*, $N, $T, $off, true, true);
        __mmio_mk_register!($e, $($t)*);
    };

    // Array
    ($e:ident, $(#[$attr:meta])* $N:ident @ $off:expr_2021 => R, $T:ty, $sz:expr_2021; $($t:tt)*) => {
        __mmio_mk_register!(@MAKE, $e, $(#[$attr])*, $N, $T, core::mem::size_of::<$T>(), $off, $sz, true, false);
        __mmio_mk_register!($e, $($t)*);
    };
    ($e:ident, $(#[$attr:meta])* $N:ident @ $off:expr_2021 => W, $T:ty, $sz:expr_2021; $($t:tt)*) => {
        __mmio_mk_register!(@MAKE, $e, $(#[$attr])*, $N, $T, core::mem::size_of::<$T>(), $off, $sz, false, true);
        __mmio_mk_register!($e, $($t)*);
    };
    ($e:ident, $(#[$attr:meta])* $N:ident @ $off:expr_2021 => RW, $T:ty, $sz:expr_2021; $($t:tt)*) => {
        __mmio_mk_register!(@MAKE, $e, $(#[$attr])*, $N, $T, core::mem::size_of::<$T>(), $off, $sz, true, true);
       __mmio_mk_register!($e, $($t)*);
    };

    // Array with Stride
    ($e:ident, $(#[$attr:meta])* $N:ident @ $off:expr_2021, $S:expr_2021 => R, $T:ty, $sz:expr_2021; $($t:tt)*) => {
        __mmio_mk_register!(@MAKE, $e, $(#[$attr])*, $N, $T, $S, $off, $sz, true, false);
        __mmio_mk_register!($e, $($t)*);
    };
    ($e:ident, $(#[$attr:meta])* $N:ident @ $off:expr_2021, $S:expr_2021 => W, $T:ty, $sz:expr_2021; $($t:tt)*) => {
        __mmio_mk_register!(@MAKE, $e, $(#[$attr])*, $N, $T, $S, $off, $sz, false, true);
        __mmio_mk_register!($e, $($t)*);
    };
    ($e:ident, $(#[$attr:meta])* $N:ident @ $off:expr_2021, $S:expr_2021 => RW, $T:ty, $sz:expr_2021; $($t:tt)*) => {
        __mmio_mk_register!(@MAKE, $e, $(#[$attr])*, $N, $T, $S, $off, $sz, true, true);
       __mmio_mk_register!($e, $($t)*);
    };
    (@MAKE, $e:ident, $(#[$attr:meta])*, $N:ident, $T: ty, $off:expr_2021, $r:expr_2021, $w:expr_2021) => {
        impl $e {
            $(#[$attr])*
            #[inline(always)]
            #[allow(non_snake_case)]
            #[allow(dead_code)]
            pub fn $N(&self) -> $crate::dev::mmio::MmioAccessor<$T, $r, $w> {
                let core::ops::Range { start, end } = self.0;
                core::assert!(
                    (start + $off) + core::mem::size_of::<$T>() <= end,
                    "{:x} {:x} {:x} {:x} {:?}",
                    start,
                    start + $off,
                    (start + $off) + core::mem::size_of::<$T>(),
                    end,
                    (start + $off) + core::mem::size_of::<$T>() <= end,
                );
                $crate::dev::mmio::MmioAccessor((start + $off) as *mut $T)
            }
        }
    };
    (@MAKE, $e:ident, $(#[$attr:meta])*, $N:ident, $T:ty, $S:expr_2021, $off:expr_2021, $sz:expr_2021, $r:expr_2021, $w:expr_2021) => {
        impl $e {
            $(#[$attr])*
            #[inline(always)]
            #[allow(non_snake_case)]
            pub fn $N(&self) -> $crate::dev::mmio::MmioArrayAccessor<$T, $r, $w, $sz> {
                let core::ops::Range { start, end } = self.0;
                core::assert!(start + $off + $S * $sz - $sz < end);

                $crate::dev::mmio::MmioArrayAccessor((start + $off) as *mut $T, $S)
            }
        }
    };
    ($e:expr_2021,) => ();
}

#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! __mmio_mk_method {
    ($N:ident) => {
        impl $N {
            /// Create new mmio area.
            pub fn new_from_mmio_area(area: $crate::dev::mmio::MmioArea) -> Self {
                let core::ops::Range { start, end } = area.0;
                Self(start.into_kva().into_usize()..end.into_kva().into_usize())
            }

            /// Get starting virtual address.
            #[inline]
            #[allow(dead_code)]
            pub fn kva(&self) -> $crate::addressing::Kva {
                $crate::addressing::Kva::new(self.0.start).unwrap()
            }
        }
    };
}

/// Make mmio register groups.
#[macro_export]
macro_rules! mmio {
    ($(#[$attr:meta])* $N:ident: $($t:tt)*) => {
        $(#[$attr])*
        struct $N(core::ops::Range<usize>);

        $crate::__mmio_mk_method!($N);
        $crate::__mmio_mk_register!($N, $($t)*);
    };

    ($(#[$attr:meta])* pub $N:ident: $($t:tt)*) => {
        $(#[$attr])*
        pub struct $N(core::ops::Range<usize>);

        $crate::__mmio_mk_method!($N);
        $crate::__mmio_mk_register!($N, $($t)*);
    };

    ($(#[$attr:meta])* pub ($($vis:tt)+) $N:ident: $($t:tt)*) => {
        $(#[$attr])*
        #[allow(non_snake_case, dead_code)]
        pub ($($vis:tt)+) struct $N(core::ops::Range<usize>);

        $crate::__mmio_mk_method!($N);
        $crate::__mmio_mk_register!($N, $($t)*);
    };
}
