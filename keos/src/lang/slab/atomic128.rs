use core::cell::UnsafeCell;
use core::fmt;
use core::intrinsics;
use core::intrinsics::AtomicOrdering;
use core::sync::atomic::Ordering;

macro_rules! atomic_int {
    (
        $atomic_type:ident,
        $int_type:ident
    ) => {
        #[repr(align(16))]
        pub struct $atomic_type {
            v: UnsafeCell<$int_type>,
        }

        impl Default for $atomic_type {
            #[inline]
            fn default() -> Self {
                Self::new(Default::default())
            }
        }

        impl From<$int_type> for $atomic_type {
            fn from(v: $int_type) -> Self {
                Self::new(v)
            }
        }

        impl fmt::Debug for $atomic_type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.load(Ordering::SeqCst), f)
            }
        }

        unsafe impl Sync for $atomic_type {}

        impl $atomic_type {
            #[inline]
            pub const fn new(v: $int_type) -> Self {
                Self {
                    v: UnsafeCell::new(v),
                }
            }

            #[inline]
            pub fn get_mut(&mut self) -> &mut $int_type {
                self.v.get_mut()
            }

            #[inline]
            pub const fn into_inner(self) -> $int_type {
                self.v.into_inner()
            }

            #[inline]
            pub fn load(&self, order: Ordering) -> $int_type {
                // SAFETY: data races are prevented by atomic intrinsics.
                unsafe { atomic_load(self.v.get(), order) }
            }

            #[inline]
            pub fn fetch_add(&self, v: $int_type, order: Ordering) -> $int_type {
                // SAFETY: data races are prevented by atomic intrinsics.
                unsafe { atomic_add(self.v.get(), v, order) }
            }

            pub fn compare_exchange(
                &self,
                current: $int_type,
                new: $int_type,
                success: Ordering,
                failure: Ordering,
            ) -> Result<$int_type, $int_type> {
                // SAFETY: data races are prevented by atomic intrinsics.
                unsafe { atomic_compare_exchange(self.v.get(), current, new, success, failure) }
            }

            #[inline]
            pub fn store(&self, val: $int_type, order: Ordering) {
                // SAFETY: data races are prevented by atomic intrinsics.
                unsafe {
                    atomic_store(self.v.get(), val, order);
                }
            }
        }
    };
}

atomic_int!(AtomicU128, u128);
atomic_int!(AtomicI128, i128);

#[inline]
unsafe fn atomic_load<T: Copy>(dst: *const T, order: Ordering) -> T {
    unsafe {
        // SAFETY: the caller must uphold the safety contract for `atomic_load`.
        match order {
            Ordering::Acquire => intrinsics::atomic_load::<T, { AtomicOrdering::Acquire }>(dst),
            Ordering::Relaxed => intrinsics::atomic_load::<T, { AtomicOrdering::Relaxed }>(dst),
            Ordering::SeqCst => intrinsics::atomic_load::<T, { AtomicOrdering::SeqCst }>(dst),
            Ordering::Release => panic!("there is no such thing as a release load"),
            Ordering::AcqRel => panic!("there is no such thing as an acquire/release load"),
            _ => core::hint::unreachable_unchecked(),
        }
    }
}

/// Returns the previous value (like __sync_fetch_and_add).
#[inline]
unsafe fn atomic_add<T: Copy>(dst: *mut T, val: T, order: Ordering) -> T {
    unsafe {
        // SAFETY: the caller must uphold the safety contract for `atomic_add`.
        match order {
            Ordering::Acquire => {
                intrinsics::atomic_xadd::<T, T, { AtomicOrdering::Acquire }>(dst, val)
            }
            Ordering::Release => {
                intrinsics::atomic_xadd::<T, T, { AtomicOrdering::Release }>(dst, val)
            }
            Ordering::AcqRel => {
                intrinsics::atomic_xadd::<T, T, { AtomicOrdering::AcqRel }>(dst, val)
            }
            Ordering::Relaxed => {
                intrinsics::atomic_xadd::<T, T, { AtomicOrdering::Relaxed }>(dst, val)
            }
            Ordering::SeqCst => {
                intrinsics::atomic_xadd::<T, T, { AtomicOrdering::SeqCst }>(dst, val)
            }
            _ => core::hint::unreachable_unchecked(),
        }
    }
}

#[inline]
unsafe fn atomic_compare_exchange<T: Copy>(
    dst: *mut T,
    old: T,
    new: T,
    success: Ordering,
    failure: Ordering,
) -> Result<T, T> {
    // SAFETY: the caller must uphold the safety contract for
    // `atomic_compare_exchange`.
    let (val, ok) = unsafe {
        match (success, failure) {
            (Ordering::Relaxed, Ordering::Relaxed) => intrinsics::atomic_cxchg::<
                T,
                { AtomicOrdering::Relaxed },
                { AtomicOrdering::Relaxed },
            >(dst, old, new),
            (Ordering::Relaxed, Ordering::Acquire) => intrinsics::atomic_cxchg::<
                T,
                { AtomicOrdering::Relaxed },
                { AtomicOrdering::Acquire },
            >(dst, old, new),
            (Ordering::Relaxed, Ordering::SeqCst) => {
                intrinsics::atomic_cxchg::<T, { AtomicOrdering::Relaxed }, { AtomicOrdering::SeqCst }>(
                    dst, old, new,
                )
            }
            (Ordering::Acquire, Ordering::Relaxed) => intrinsics::atomic_cxchg::<
                T,
                { AtomicOrdering::Acquire },
                { AtomicOrdering::Relaxed },
            >(dst, old, new),
            (Ordering::Acquire, Ordering::Acquire) => intrinsics::atomic_cxchg::<
                T,
                { AtomicOrdering::Acquire },
                { AtomicOrdering::Acquire },
            >(dst, old, new),
            (Ordering::Acquire, Ordering::SeqCst) => {
                intrinsics::atomic_cxchg::<T, { AtomicOrdering::Acquire }, { AtomicOrdering::SeqCst }>(
                    dst, old, new,
                )
            }
            (Ordering::Release, Ordering::Relaxed) => intrinsics::atomic_cxchg::<
                T,
                { AtomicOrdering::Release },
                { AtomicOrdering::Relaxed },
            >(dst, old, new),
            (Ordering::Release, Ordering::Acquire) => intrinsics::atomic_cxchg::<
                T,
                { AtomicOrdering::Release },
                { AtomicOrdering::Acquire },
            >(dst, old, new),
            (Ordering::Release, Ordering::SeqCst) => {
                intrinsics::atomic_cxchg::<T, { AtomicOrdering::Release }, { AtomicOrdering::SeqCst }>(
                    dst, old, new,
                )
            }
            (Ordering::AcqRel, Ordering::Relaxed) => {
                intrinsics::atomic_cxchg::<T, { AtomicOrdering::AcqRel }, { AtomicOrdering::Relaxed }>(
                    dst, old, new,
                )
            }
            (Ordering::AcqRel, Ordering::Acquire) => {
                intrinsics::atomic_cxchg::<T, { AtomicOrdering::AcqRel }, { AtomicOrdering::Acquire }>(
                    dst, old, new,
                )
            }
            (Ordering::AcqRel, Ordering::SeqCst) => {
                intrinsics::atomic_cxchg::<T, { AtomicOrdering::AcqRel }, { AtomicOrdering::SeqCst }>(
                    dst, old, new,
                )
            }
            (Ordering::SeqCst, Ordering::Relaxed) => {
                intrinsics::atomic_cxchg::<T, { AtomicOrdering::SeqCst }, { AtomicOrdering::Relaxed }>(
                    dst, old, new,
                )
            }
            (Ordering::SeqCst, Ordering::Acquire) => {
                intrinsics::atomic_cxchg::<T, { AtomicOrdering::SeqCst }, { AtomicOrdering::Acquire }>(
                    dst, old, new,
                )
            }
            (Ordering::SeqCst, Ordering::SeqCst) => {
                intrinsics::atomic_cxchg::<T, { AtomicOrdering::SeqCst }, { AtomicOrdering::SeqCst }>(
                    dst, old, new,
                )
            }
            (_, Ordering::AcqRel) => {
                panic!("there is no such thing as an acquire-release failure ordering")
            }
            (_, Ordering::Release) => {
                panic!("there is no such thing as a release failure ordering")
            }
            (_, _) => unreachable!(),
        }
    };
    if ok { Ok(val) } else { Err(val) }
}

#[inline]
unsafe fn atomic_store<T: Copy>(dst: *mut T, val: T, order: Ordering) {
    unsafe {
        // SAFETY: the caller must uphold the safety contract for `atomic_store`.
        match order {
            Ordering::Release => {
                intrinsics::atomic_store::<T, { AtomicOrdering::Release }>(dst, val)
            }
            Ordering::Relaxed => {
                intrinsics::atomic_store::<T, { AtomicOrdering::Relaxed }>(dst, val)
            }
            Ordering::SeqCst => intrinsics::atomic_store::<T, { AtomicOrdering::SeqCst }>(dst, val),
            Ordering::Acquire => panic!("there is no such thing as an acquire store"),
            Ordering::AcqRel => panic!("there is no such thing as an acquire/release store"),
            _ => core::hint::unreachable_unchecked(),
        }
    }
}
