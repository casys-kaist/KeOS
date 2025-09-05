//! A wrapper around the core::sync::atomic.
//!
//! # Atomic types
//!
//! Atomic types provide primitive shared-memory communication between
//! threads, and are the building blocks of other concurrent
//! types.
//!
//! This module defines a wrapper to the atomic types defined in
//! [`core::sync::atomic`], including [`AtomicBool`], [`AtomicIsize`],
//! [`AtomicUsize`], [`AtomicI8`], [`AtomicU16`], etc.
//! Atomic types present operations that, when used correctly, synchronize
//! updates between threads.
//!
//! Atomic variables are safe to share between threads (they implement [`Sync`])
//! but they do not themselves provide the mechanism for sharing and follow the
//! [threading model](https://doc.rust-lang.org/std/thread/#the-threading-model) of
//! Rust. The most common way to share an atomic variable is to put it into an
//! [`Arc`][arc] (an atomically-reference-counted shared pointer).
//!
//! [arc]: https://doc.rust-lang.org/beta/std/sync/struct.Arc.html
//! [`core::sync::atomic`]: https://doc.rust-lang.org/core/sync/atomic/
//! [`Sync`]: https://doc.rust-lang.org/beta/core/marker/trait.Sync.html

use core::sync::atomic::Ordering;

/// A boolean type which can be safely shared between threads.
///
/// This type has the same size, alignment, and bit validity as a [`bool`].
#[derive(Default)]
pub struct AtomicBool(core::sync::atomic::AtomicBool);

/// A raw pointer type which can be safely shared between threads.
///
/// This type has the same size and bit validity as a `*mut T`.
///
/// **Note**: This type is only available on platforms that support atomic
/// loads and stores of pointers. Its size depends on the target pointer's size.
#[derive(Default)]
pub struct AtomicPtr<T>(core::sync::atomic::AtomicPtr<T>);

impl AtomicBool {
    /// Creates a new `AtomicBool`.
    ///
    /// # Examples
    ///
    /// ```
    /// use KeOS::sync::atomic::AtomicBool;
    ///
    /// let atomic_true = AtomicBool::new(true);
    /// let atomic_false = AtomicBool::new(false);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new(v: bool) -> AtomicBool {
        Self(core::sync::atomic::AtomicBool::new(v))
    }

    /// Returns a mutable reference to the underlying [`bool`].
    ///
    /// This is safe because the mutable reference guarantees that no other
    /// threads are concurrently accessing the atomic data.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicBool;
    ///
    /// let mut some_bool = AtomicBool::new(true);
    /// assert_eq!(*some_bool.get_mut(), true);
    /// *some_bool.get_mut() = false;
    /// assert_eq!(some_bool.load(), false);
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> &mut bool {
        self.0.get_mut()
    }

    /// Consumes the atomic and returns the contained value.
    ///
    /// This is safe because passing `self` by value guarantees that no other
    /// threads are concurrently accessing the atomic data.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicBool;
    ///
    /// let some_bool = AtomicBool::new(true);
    /// assert_eq!(some_bool.into_inner(), true);
    /// ```
    #[inline]
    pub const fn into_inner(self) -> bool {
        self.0.into_inner()
    }

    /// Loads a value from the bool.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicBool;
    ///
    /// let some_bool = AtomicBool::new(true);
    ///
    /// assert_eq!(some_bool.load(), true);
    /// ```
    #[inline]
    pub fn load(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    /// Stores a value into the bool.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicBool;
    ///
    /// let some_bool = AtomicBool::new(true);
    ///
    /// some_bool.store(false);
    /// assert_eq!(some_bool.load(), false);
    /// ```
    #[inline]
    pub fn store(&self, val: bool) {
        self.0.store(val, Ordering::SeqCst)
    }

    /// Stores a value into the bool, returning the previous value.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicBool;
    ///
    /// let some_bool = AtomicBool::new(true);
    ///
    /// assert_eq!(some_bool.swap(false), true);
    /// assert_eq!(some_bool.load(), false);
    /// ```
    #[inline]
    pub fn swap(&self, val: bool) -> bool {
        self.0.swap(val, Ordering::SeqCst)
    }

    /// Stores a value into the [`bool`] if the current value is the same as the
    /// `current` value.
    ///
    /// The return value is a result indicating whether the new value was
    /// written and containing the previous value. On success this value is
    /// guaranteed to be equal to `current`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::atomic::AtomicBool;
    ///
    /// let some_bool = AtomicBool::new(true);
    ///
    /// assert_eq!(some_bool.compare_exchange(true, false), Ok(true));
    /// assert_eq!(some_bool.load(), false);
    ///
    /// assert_eq!(some_bool.compare_exchange(true, true), Err(false));
    /// assert_eq!(some_bool.load(), false);
    /// ```
    #[inline]
    pub fn compare_exchange(&self, current: bool, new: bool) -> Result<bool, bool> {
        self.0
            .compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst)
    }

    /// Logical "and" with a boolean value.
    ///
    /// Performs a logical "and" operation on the current value and the argument
    /// `val`, and sets the new value to the result.
    ///
    /// Returns the previous value.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicBool;
    ///
    /// let foo = AtomicBool::new(true);
    /// assert_eq!(foo.fetch_and(false), true);
    /// assert_eq!(foo.load(), false);
    ///
    /// let foo = AtomicBool::new(true);
    /// assert_eq!(foo.fetch_and(true), true);
    /// assert_eq!(foo.load(), true);
    ///
    /// let foo = AtomicBool::new(false);
    /// assert_eq!(foo.fetch_and(false), false);
    /// assert_eq!(foo.load(), false);
    /// ```
    #[inline]
    pub fn fetch_and(&self, val: bool) -> bool {
        self.0.fetch_and(val, Ordering::SeqCst)
    }

    /// Logical "nand" with a boolean value.
    ///
    /// Performs a logical "nand" operation on the current value and the
    /// argument `val`, and sets the new value to the result.
    ///
    /// Returns the previous value.
    ///
    /// **Note:** This method is only available on platforms that support atomic
    /// operations on `u8`.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::{AtomicBool, Ordering};
    ///
    /// let foo = AtomicBool::new(true);
    /// assert_eq!(foo.fetch_nand(false), true);
    /// assert_eq!(foo.load(), true);
    ///
    /// let foo = AtomicBool::new(true);
    /// assert_eq!(foo.fetch_nand(true), true);
    /// assert_eq!(foo.load() as usize, 0);
    /// assert_eq!(foo.load(), false);
    ///
    /// let foo = AtomicBool::new(false);
    /// assert_eq!(foo.fetch_nand(false), false);
    /// assert_eq!(foo.load(), true);
    /// ```
    #[inline]
    pub fn fetch_nand(&self, val: bool) -> bool {
        self.0.fetch_nand(val, Ordering::SeqCst)
    }

    /// Logical "or" with a boolean value.
    ///
    /// Performs a logical "or" operation on the current value and the argument
    /// `val`, and sets the new value to the result.
    ///
    /// Returns the previous value.
    ///
    /// **Note:** This method is only available on platforms that support atomic
    /// operations on `u8`.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::{AtomicBool, Ordering};
    ///
    /// let foo = AtomicBool::new(true);
    /// assert_eq!(foo.fetch_or(false), true);
    /// assert_eq!(foo.load(), true);
    ///
    /// let foo = AtomicBool::new(true);
    /// assert_eq!(foo.fetch_or(true), true);
    /// assert_eq!(foo.load(), true);
    ///
    /// let foo = AtomicBool::new(false);
    /// assert_eq!(foo.fetch_or(false), false);
    /// assert_eq!(foo.load(), false);
    /// ```
    #[inline]
    pub fn fetch_or(&self, val: bool) -> bool {
        self.0.fetch_or(val, Ordering::SeqCst)
    }

    /// Logical "xor" with a boolean value.
    ///
    /// Performs a logical "xor" operation on the current value and the argument
    /// `val`, and sets the new value to the result.
    ///
    /// Returns the previous value.
    ///
    /// **Note:** This method is only available on platforms that support atomic
    /// operations on `u8`.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::{AtomicBool, Ordering};
    ///
    /// let foo = AtomicBool::new(true);
    /// assert_eq!(foo.fetch_xor(false), true);
    /// assert_eq!(foo.load(), true);
    ///
    /// let foo = AtomicBool::new(true);
    /// assert_eq!(foo.fetch_xor(true), true);
    /// assert_eq!(foo.load(), false);
    ///
    /// let foo = AtomicBool::new(false);
    /// assert_eq!(foo.fetch_xor(false), false);
    /// assert_eq!(foo.load(), false);
    /// ```
    #[inline]
    pub fn fetch_xor(&self, val: bool) -> bool {
        self.0.fetch_xor(val, Ordering::SeqCst)
    }

    /// Logical "not" with a boolean value.
    ///
    /// Performs a logical "not" operation on the current value, and sets
    /// the new value to the result.
    ///
    /// Returns the previous value.
    ///
    /// **Note:** This method is only available on platforms that support atomic
    /// operations on `u8`.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicBool;
    ///
    /// let foo = AtomicBool::new(true);
    /// assert_eq!(foo.fetch_not(), true);
    /// assert_eq!(foo.load(), false);
    ///
    /// let foo = AtomicBool::new(false);
    /// assert_eq!(foo.fetch_not(), false);
    /// assert_eq!(foo.load(), true);
    /// ```
    #[inline]
    pub fn fetch_not(&self) -> bool {
        self.0.fetch_not(Ordering::SeqCst)
    }

    /// Fetches the value, and applies a function to it that returns an optional
    /// new value. Returns a `Result` of `Ok(previous_value)` if the function
    /// returned `Some(_)`, else `Err(previous_value)`.
    ///
    /// Note: This may call the function multiple times if the value has been
    /// changed from other threads in the meantime, as long as the function
    /// returns `Some(_)`, but the function will have been applied only once to
    /// the stored value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use keos::atomic::AtomicBool;
    ///
    /// let x = AtomicBool::new(false);
    /// assert_eq!(x.fetch_update(|_| None), Err(false));
    /// assert_eq!(x.fetch_update(|x| Some(!x)), Ok(false));
    /// assert_eq!(x.fetch_update(|x| Some(!x)), Ok(true));
    /// assert_eq!(x.load(), false);
    /// ```
    #[inline]
    pub fn fetch_update<F>(&self, f: F) -> Result<bool, bool>
    where
        F: FnMut(bool) -> Option<bool>,
    {
        self.0.fetch_update(Ordering::SeqCst, Ordering::SeqCst, f)
    }
}

impl<T> AtomicPtr<T> {
    /// Creates a new `AtomicPtr`.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicPtr;
    ///
    /// let ptr = &mut 5;
    /// let atomic_ptr = AtomicPtr::new(ptr);
    /// ```
    #[inline]
    pub const fn new(p: *mut T) -> AtomicPtr<T> {
        Self(core::sync::atomic::AtomicPtr::new(p))
    }

    /// Returns a mutable reference to the underlying pointer.
    ///
    /// This is safe because the mutable reference guarantees that no other
    /// threads are concurrently accessing the atomic data.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicPtr;
    ///
    /// let mut data = 10;
    /// let mut atomic_ptr = AtomicPtr::new(&mut data);
    /// let mut other_data = 5;
    /// *atomic_ptr.get_mut() = &mut other_data;
    /// assert_eq!(unsafe { *atomic_ptr.load() }, 5);
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> &mut *mut T {
        self.0.get_mut()
    }

    /// Consumes the atomic and returns the contained value.
    ///
    /// This is safe because passing `self` by value guarantees that no other
    /// threads are concurrently accessing the atomic data.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicPtr;
    ///
    /// let mut data = 5;
    /// let atomic_ptr = AtomicPtr::new(&mut data);
    /// assert_eq!(unsafe { *atomic_ptr.into_inner() }, 5);
    /// ```
    #[inline]
    pub const fn into_inner(self) -> *mut T {
        self.0.into_inner()
    }

    /// Loads a value from the pointer.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicPtr;
    ///
    /// let ptr = &mut 5;
    /// let some_ptr = AtomicPtr::new(ptr);
    ///
    /// let value = some_ptr.load();
    /// ```
    #[inline]
    pub fn load(&self) -> *mut T {
        self.0.load(Ordering::SeqCst)
    }

    /// Stores a value into the pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicPtr;
    ///
    /// let ptr = &mut 5;
    /// let some_ptr = AtomicPtr::new(ptr);
    ///
    /// let other_ptr = &mut 10;
    ///
    /// some_ptr.store(other_ptr);
    /// ```
    #[inline]
    pub fn store(&self, ptr: *mut T) {
        self.0.store(ptr, Ordering::SeqCst);
    }

    /// Stores a value into the pointer, returning the previous value.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicPtr;
    ///
    /// let ptr = &mut 5;
    /// let some_ptr = AtomicPtr::new(ptr);
    ///
    /// let other_ptr = &mut 10;
    ///
    /// let value = some_ptr.swap(other_ptr);
    /// ```
    #[inline]
    pub fn swap(&self, ptr: *mut T) -> *mut T {
        self.0.swap(ptr, Ordering::SeqCst)
    }

    /// Stores a value into the pointer if the current value is the same as the
    /// `current` value.
    ///
    /// The return value is a result indicating whether the new value was
    /// written and containing the previous value. On success this value is
    /// guaranteed to be equal to `current`.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicPtr;
    ///
    /// let ptr = &mut 5;
    /// let some_ptr = AtomicPtr::new(ptr);
    ///
    /// let other_ptr = &mut 10;
    ///
    /// let value = some_ptr.compare_exchange(ptr, other_ptr);
    /// ```
    #[inline]
    pub fn compare_exchange(&self, current: *mut T, new: *mut T) -> Result<*mut T, *mut T> {
        self.0
            .compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst)
    }

    /// Fetches the value, and applies a function to it that returns an optional
    /// new value. Returns a `Result` of `Ok(previous_value)` if the function
    /// returned `Some(_)`, else `Err(previous_value)`.
    ///
    /// Note: This may call the function multiple times if the value has been
    /// changed from other threads in the meantime, as long as the function
    /// returns `Some(_)`, but the function will have been applied only once to
    /// the stored value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use keos::atomic::AtomicPtr;
    ///
    /// let ptr: *mut _ = &mut 5;
    /// let some_ptr = AtomicPtr::new(ptr);
    ///
    /// let new: *mut _ = &mut 10;
    /// assert_eq!(some_ptr.fetch_update(, |_| None), Err(ptr));
    /// let result = some_ptr.fetch_update(, |x| {
    ///     if x == ptr {
    ///         Some(new)
    ///     } else {
    ///         None
    ///     }
    /// });
    /// assert_eq!(result, Ok(ptr));
    /// assert_eq!(some_ptr.load(), new);
    /// ```
    #[inline]
    pub fn fetch_update<F>(&self, f: F) -> Result<*mut T, *mut T>
    where
        F: FnMut(*mut T) -> Option<*mut T>,
    {
        self.0.fetch_update(Ordering::SeqCst, Ordering::SeqCst, f)
    }

    /// Returns a mutable pointer to the underlying pointer.
    ///
    /// Doing non-atomic reads and writes on the resulting pointer can be a data
    /// race. This method is mostly useful for FFI, where the function
    /// signature may use `*mut *mut T` instead of `&AtomicPtr<T>`.
    ///
    /// Returning an `*mut` pointer from a shared reference to this atomic is
    /// safe because the atomic types work with interior mutability. All
    /// modifications of an atomic change the value through a shared
    /// reference, and can do so safely as long as they use atomic operations.
    /// Any use of the returned raw pointer requires an `unsafe` block and
    /// still has to uphold the same restriction: operations on it must be
    /// atomic.
    ///
    /// # Examples
    ///
    /// ```ignore (extern-declaration)
    /// use keos::atomic::AtomicPtr;
    ///
    /// extern "C" {
    ///     fn my_atomic_op(arg: *mut *mut u32);
    /// }
    ///
    /// let mut value = 17;
    /// let atomic = AtomicPtr::new(&mut value);
    ///
    /// // SAFETY: Safe as long as `my_atomic_op` is atomic.
    /// unsafe {
    ///     my_atomic_op(atomic.as_ptr());
    /// }
    /// ```
    #[inline]
    pub const fn as_ptr(&self) -> *mut *mut T {
        self.0.as_ptr()
    }
}

impl From<bool> for AtomicBool {
    /// Converts a `bool` into an `AtomicBool`.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::atomic::AtomicBool;
    /// let atomic_bool = AtomicBool::from(true);
    /// assert_eq!(format!("{atomic_bool:?}"), "true")
    /// ```
    #[inline]
    fn from(b: bool) -> Self {
        Self::new(b)
    }
}

impl<T> From<*mut T> for AtomicPtr<T> {
    /// Converts a `*mut T` into an `AtomicPtr<T>`.
    #[inline]
    fn from(p: *mut T) -> Self {
        Self::new(p)
    }
}

#[allow(unused_macros)] // This macro ends up being unused on some architectures.
macro_rules! if_8_bit {
    (u8, $( yes = [$($yes:tt)*], )? $( no = [$($no:tt)*], )? ) => { concat!("", $($($yes)*)?) };
    (i8, $( yes = [$($yes:tt)*], )? $( no = [$($no:tt)*], )? ) => { concat!("", $($($yes)*)?) };
    ($_:ident, $( yes = [$($yes:tt)*], )? $( no = [$($no:tt)*], )? ) => { concat!("", $($($no)*)?) };
}

macro_rules! atomic_int {
    ($s_int_type:literal,
     $extra_feature:expr,
     $int_type:ident $atomic_type:ident) => {
        /// An integer type which can be safely shared between threads.
        ///
        /// This type has the same
        #[doc = if_8_bit!(
            $int_type,
            yes = ["size, alignment, and bit validity"],
            no = ["size and bit validity"],
        )]
        /// as the underlying integer type, [`
        #[doc = $s_int_type]
        /// `].
        #[doc = if_8_bit! {
            $int_type,
            no = [
                "However, the alignment of this type is always equal to its ",
                "size, even on targets where [`", $s_int_type, "`] has a ",
                "lesser alignment."
            ],
        }]
        ///
        /// For more about the differences between atomic types and
        /// non-atomic types as well as information about the portability of
        /// this type, please see the [module-level documentation].
        ///
        /// **Note:** This type is only available on platforms that support
        /// atomic loads and stores of [`
        #[doc = $s_int_type]
        /// `].
        ///
        #[repr(transparent)]
        #[derive(Default)]
        pub struct $atomic_type(core::sync::atomic::$atomic_type);

        impl From<$int_type> for $atomic_type {
            #[doc = concat!("Converts an `", stringify!($int_type), "` into an `", stringify!($atomic_type), "`.")]
            #[inline]
            fn from(v: $int_type) -> Self { Self::new(v) }
        }

        impl core::fmt::Debug for $atomic_type {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Debug::fmt(&self.load(), f)
            }
        }

        impl $atomic_type {
            /// Creates a new atomic integer.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let atomic_forty_two = ", stringify!($atomic_type), "::new(42);")]
            /// ```
            #[inline]
            #[must_use]
            pub const fn new(v: $int_type) -> Self {
                Self(core::sync::atomic::$atomic_type::new(v))
            }

            /// Returns a mutable reference to the underlying integer.
            ///
            /// This is safe because the mutable reference guarantees that no other threads are
            /// concurrently accessing the atomic data.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let mut some_var = ", stringify!($atomic_type), "::new(10);")]
            /// assert_eq!(*some_var.get_mut(), 10);
            /// *some_var.get_mut() = 5;
            /// assert_eq!(some_var.load(), 5);
            /// ```
            #[inline]
            pub fn get_mut(&mut self) -> &mut $int_type {
                self.0.get_mut()
            }

            /// Consumes the atomic and returns the contained value.
            ///
            /// This is safe because passing `self` by value guarantees that no other threads are
            /// concurrently accessing the atomic data.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let some_var = ", stringify!($atomic_type), "::new(5);")]
            /// assert_eq!(some_var.into_inner(), 5);
            /// ```
            #[inline]
            pub const fn into_inner(self) -> $int_type {
                self.0.into_inner()
            }

            /// Loads a value from the atomic integer.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let some_var = ", stringify!($atomic_type), "::new(5);")]
            ///
            /// assert_eq!(some_var.load(d), 5);
            /// ```
            #[inline]
            pub fn load(&self) -> $int_type {
                self.0.load(Ordering::SeqCst)
            }

            /// Stores a value into the atomic integer.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let some_var = ", stringify!($atomic_type), "::new(5);")]
            ///
            /// some_var.store(10d);
            /// assert_eq!(some_var.load(d), 10);
            /// ```
            #[inline]
            pub fn store(&self, val: $int_type) {
                self.0.store(val, Ordering::SeqCst)
            }

            /// Stores a value into the atomic integer, returning the previous value.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let some_var = ", stringify!($atomic_type), "::new(5);")]
            ///
            /// assert_eq!(some_var.swap(10d), 5);
            /// ```
            #[inline]
            pub fn swap(&self, val: $int_type) -> $int_type {
                self.0.swap(val, Ordering::SeqCst)
            }

            /// Stores a value into the atomic integer if the current value is the same as
            /// the `current` value.
            ///
            /// The return value is a result indicating whether the new value was written and
            /// containing the previous value. On success this value is guaranteed to be equal to
            /// `current`.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let some_var = ", stringify!($atomic_type), "::new(5);")]
            ///
            /// assert_eq!(some_var.compare_exchange(5, 10, e), Ok(5));
            /// assert_eq!(some_var.load(d), 10);
            ///
            /// assert_eq!(some_var.compare_exchange(6, 12, e), Err(10));
            /// assert_eq!(some_var.load(d), 10);
            /// ```
            #[inline]
            pub fn compare_exchange(&self,
                                    current: $int_type,
                                    new: $int_type) -> Result<$int_type, $int_type> {
                self.0.compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst)
            }


            /// Adds to the current value, returning the previous value.
            ///
            /// This operation wraps around on overflow.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let foo = ", stringify!($atomic_type), "::new(0);")]
            /// assert_eq!(foo.fetch_add(10), 0);
            /// assert_eq!(foo.load(), 10);
            /// ```
            #[inline]
            pub fn fetch_add(&self, val: $int_type) -> $int_type {
                self.0.fetch_add(val, Ordering::SeqCst)
            }

            /// Subtracts from the current value, returning the previous value.
            ///
            /// This operation wraps around on overflow.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let foo = ", stringify!($atomic_type), "::new(20);")]
            /// assert_eq!(foo.fetch_sub(10), 20);
            /// assert_eq!(foo.load(), 10);
            /// ```
            #[inline]
            pub fn fetch_sub(&self, val: $int_type) -> $int_type {
                self.0.fetch_sub(val, Ordering::SeqCst)
            }

            /// Bitwise "and" with the current value.
            ///
            /// Performs a bitwise "and" operation on the current value and the argument `val`, and
            /// sets the new value to the result.
            ///
            /// Returns the previous value.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let foo = ", stringify!($atomic_type), "::new(0b101101);")]
            /// assert_eq!(foo.fetch_and(0b110011), 0b101101);
            /// assert_eq!(foo.load(), 0b100001);
            /// ```
            #[inline]
            pub fn fetch_and(&self, val: $int_type) -> $int_type {
                self.0.fetch_and(val, Ordering::SeqCst)
            }

            /// Bitwise "nand" with the current value.
            ///
            /// Performs a bitwise "nand" operation on the current value and the argument `val`, and
            /// sets the new value to the result.
            ///
            /// Returns the previous value.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let foo = ", stringify!($atomic_type), "::new(0x13);")]
            /// assert_eq!(foo.fetch_nand(0x31), 0x13);
            /// assert_eq!(foo.load(), !(0x13 & 0x31));
            /// ```
            #[inline]
            pub fn fetch_nand(&self, val: $int_type) -> $int_type {
                self.0.fetch_nand(val, Ordering::SeqCst)
            }

            /// Bitwise "or" with the current value.
            ///
            /// Performs a bitwise "or" operation on the current value and the argument `val`, and
            /// sets the new value to the result.
            ///
            /// Returns the previous value.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let foo = ", stringify!($atomic_type), "::new(0b101101);")]
            /// assert_eq!(foo.fetch_or(0b110011), 0b101101);
            /// assert_eq!(foo.load(), 0b111111);
            /// ```
            #[inline]
            pub fn fetch_or(&self, val: $int_type) -> $int_type {
                self.0.fetch_or(val, Ordering::SeqCst)
            }

            /// Bitwise "xor" with the current value.
            ///
            /// Performs a bitwise "xor" operation on the current value and the argument `val`, and
            /// sets the new value to the result.
            ///
            /// Returns the previous value.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let foo = ", stringify!($atomic_type), "::new(0b101101);")]
            /// assert_eq!(foo.fetch_xor(0b110011), 0b101101);
            /// assert_eq!(foo.load(), 0b011110);
            /// ```
            #[inline]
            pub fn fetch_xor(&self, val: $int_type) -> $int_type {
                self.0.fetch_xor(val, Ordering::SeqCst)
            }

            /// Fetches the value, and applies a function to it that returns an optional
            /// new value. Returns a `Result` of `Ok(previous_value)` if the function returned `Some(_)`, else
            /// `Err(previous_value)`.
            ///
            /// Note: This may call the function multiple times if the value has been changed from other threads in
            /// the meantime, as long as the function returns `Some(_)`, but the function will have been applied
            /// only once to the stored value.
            ///
            /// # Examples
            ///
            /// ```rust
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let x = ", stringify!($atomic_type), "::new(7);")]
            /// assert_eq!(x.fetch_update(|_| None), Err(7));
            /// assert_eq!(x.fetch_update(|x| Some(x + 1)), Ok(7));
            /// assert_eq!(x.fetch_update(|x| Some(x + 1)), Ok(8));
            /// assert_eq!(x.load(), 9);
            /// ```
            #[inline]
            pub fn fetch_update<F>(&self, f: F) -> Result<$int_type, $int_type>
            where F: FnMut($int_type) -> Option<$int_type> {
                self.0.fetch_update(Ordering::SeqCst, Ordering::SeqCst, f)
            }

            /// Maximum with the current value.
            ///
            /// Finds the maximum of the current value and the argument `val`, and
            /// sets the new value to the result.
            ///
            /// Returns the previous value.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let foo = ", stringify!($atomic_type), "::new(23);")]
            /// assert_eq!(foo.fetch_max(42), 23);
            /// assert_eq!(foo.load(), 42);
            /// ```
            ///
            /// If you want to obtain the maximum value in one step, you can use the following:
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let foo = ", stringify!($atomic_type), "::new(23);")]
            /// let bar = 42;
            /// let max_foo = foo.fetch_max(bar).max(bar);
            /// assert!(max_foo == 42);
            /// ```
            #[inline]
            pub fn fetch_max(&self, val: $int_type) -> $int_type {
                self.0.fetch_max(val, Ordering::SeqCst)
            }

            /// Minimum with the current value.
            ///
            /// Finds the minimum of the current value and the argument `val`, and
            /// sets the new value to the result.
            ///
            /// Returns the previous value.
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let foo = ", stringify!($atomic_type), "::new(23);")]
            /// assert_eq!(foo.fetch_min(42d), 23);
            /// assert_eq!(foo.load(d), 23);
            /// assert_eq!(foo.fetch_min(22d), 23);
            /// assert_eq!(foo.load(d), 22);
            /// ```
            ///
            /// If you want to obtain the minimum value in one step, you can use the following:
            ///
            /// ```
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            #[doc = concat!("let foo = ", stringify!($atomic_type), "::new(23);")]
            /// let bar = 12;
            /// let min_foo = foo.fetch_min(bar).min(bar);
            /// assert_eq!(min_foo, 12);
            /// ```
            #[inline]
            pub fn fetch_min(&self, val: $int_type) -> $int_type {
                self.0.fetch_min(val, Ordering::SeqCst)
            }

            /// Returns a mutable pointer to the underlying integer.
            ///
            /// Doing non-atomic reads and writes on the resulting integer can be a data race.
            /// This method is mostly useful for FFI, where the function signature may use
            #[doc = concat!("`*mut ", stringify!($int_type), "` instead of `&", stringify!($atomic_type), "`.")]
            ///
            /// Returning an `*mut` pointer from a shared reference to this atomic is safe because the
            /// atomic types work with interior mutability. All modifications of an atomic change the value
            /// through a shared reference, and can do so safely as long as they use atomic operations. Any
            /// use of the returned raw pointer requires an `unsafe` block and still has to uphold the same
            /// restriction: operations on it must be atomic.
            ///
            /// # Examples
            ///
            /// ```ignore (extern-declaration)
            /// # fn main() {
            #[doc = concat!($extra_feature, "use keos::atomic::", stringify!($atomic_type), ";")]
            ///
            /// extern "C" {
            #[doc = concat!("    fn my_atomic_op(arg: *mut ", stringify!($int_type), ");")]
            /// }
            ///
            #[doc = concat!("let atomic = ", stringify!($atomic_type), "::new(1);")]
            ///
            /// // SAFETY: Safe as long as `my_atomic_op` is atomic.
            /// unsafe {
            ///     my_atomic_op(atomic.as_ptr());
            /// }
            /// # }
            /// ```
            #[inline]
            pub const fn as_ptr(&self) -> *mut $int_type {
                self.0.as_ptr()
            }
        }
    }
}

atomic_int! {
    "i8",
    "",
    i8 AtomicI8
}
atomic_int! {
    "u8",
    "",
    u8 AtomicU8
}
atomic_int! {
   "i16",
    "",
    i16 AtomicI16
}
atomic_int! {
   "u16",
    "",
    u16 AtomicU16
}
atomic_int! {
    "i32",
    "",
    i32 AtomicI32
}
atomic_int! {
    "u32",
    "",
    u32 AtomicU32
}
atomic_int! {
    "i64",
    "",
    i64 AtomicI64
}
atomic_int! {
    "u64",
    "",
    u64 AtomicU64
}

atomic_int! {
    "isize",
    "",
    isize AtomicIsize
}

atomic_int! {
    "usize",
    "",
    usize AtomicUsize
}
