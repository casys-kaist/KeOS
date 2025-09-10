//! SMP-supported spinlock.

use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

/// The lock could not be acquired at this time because the operation would
/// otherwise block.
pub struct WouldBlock;

/// A mutual exclusion primitive useful for protecting shared data
///
/// This spinlock will block threads waiting for the lock to become available.
/// The spinlock can be created via a [`new`] constructor. Each spinlock has a
/// type parameter which represents the data that it is protecting. The data can
/// only be accessed through the guards returned from [`lock`] and
/// [`try_lock`], which guarantees that the data is only ever accessed when the
/// spinlock is locked.
///
/// [`new`]: Self::new
/// [`lock`]: Self::lock
/// [`try_lock`]: Self::try_lock
/// [`unwrap()`]: Result::unwrap
///
/// # Examples
///
/// ```
/// use alloc::sync::Arc;
/// use keos::sync::SpinLock;
/// use keos::thread;
///
/// const N: usize = 10;
///
/// // Spawn a few threads to increment a shared variable (non-atomically), and
/// // let the main thread know once all increments are done.
/// //
/// // Here we're using an Arc to share memory among threads, and the data inside
/// // the Arc is protected with a spinlock.
/// let data = Arc::new(SpinLock::new(0));
///
/// for _ in 0..N {
///     let data = Arc::clone(&data);
///     thread::ThreadBuilder::new("work").spawn(move || {
///         // The shared state can only be accessed once the lock is held.
///         // Our non-atomic increment is safe because we're the only thread
///         // which can access the shared state when the lock is held.
///         //
///         // We unwrap() the return value to assert that we are not expecting
///         // threads to ever fail while holding the lock.
///         let mut guard = data.lock();
///         guard += 1;
///         // the lock must be "explicitly" unlocked before `guard` goes out of scope.
///         guard.unlock();
///     });
/// }
pub struct SpinLock<T: ?Sized> {
    locked: AtomicBool,
    _pad: [u8; 15],
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Creates a new spinlock in an unlocked state ready for use.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::sync::SpinLock;
    ///
    /// let spinlock = SpinLock::new(0);
    /// ```
    #[inline]
    pub const fn new(t: T) -> SpinLock<T> {
        SpinLock {
            data: UnsafeCell::new(t),
            _pad: [0u8; 15],
            locked: AtomicBool::new(false),
        }
    }
}

impl<T: ?Sized> SpinLock<T> {
    /// Acquires a spinlock, blocking the current thread until it is able to do
    /// so.
    ///
    /// This function will block the local thread until it is available to
    /// acquire the spinlock. Upon returning, the thread is the only thread
    /// with the lock held. An guard is returned to allow scoped access
    /// of the lock. When the guard goes out of scope without
    /// [`SpinLockGuard::unlock`], panic occurs.
    ///
    /// The exact behavior on locking a spinlock in the thread which already
    /// holds the lock is left unspecified. However, this function will not
    /// return on the second call (it might panic or deadlock, for example).
    ///
    /// # Examples
    ///
    /// ```
    /// use alloc::sync::Arc;
    /// use keos::sync::SpinLock;
    /// use keos::thread;
    ///
    /// let spinlock = Arc::new(SpinLock::new(0));
    /// let c_spinlock = Arc::clone(&spinlock);
    ///
    /// thread::spawn(move || {
    ///     let mut guard = c_spinlock.lock();
    ///     *guard = 10;
    ///     guard.unlock();
    /// }).join().expect("thread::spawn failed");
    /// let guard = spinlock.lock();
    /// assert_eq!(*guard, 10);
    /// guard.unlock();
    /// ```
    #[track_caller]
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        let guard = loop {
            let guard = crate::interrupt::InterruptGuard::new();

            core::hint::spin_loop();
            if !self.locked.fetch_or(true, Ordering::SeqCst) {
                break guard;
            }

            drop(guard);
        };

        SpinLockGuard {
            caller: core::panic::Location::caller(),
            lock: self,
            guard: Some(guard),
        }
    }
    /// Attempts to acquire this lock.
    ///
    /// If the lock could not be acquired at this time, then [`Err`] is
    /// returned. Otherwise, an guard is returned. The lock will be
    /// unlocked when the guard is dropped.
    ///
    /// This function does not block.
    ///
    /// # Errors
    ///
    /// If the spinlock could not be acquired because it is already locked, then
    /// this call will return the [`WouldBlock`] error.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::sync::SpinLock;
    /// use alloc::sync::Arc;
    /// use keos::thread;
    ///
    /// let spinlock = Arc::new(SpinLock::new(0));
    /// let c_spinlock = Arc::clone(&spinlock);
    ///
    /// thread::spawn(move || {
    ///     let mut lock = c_spinlock.try_lock();
    ///     if let Ok(ref mut spinlock) = lock {
    ///         **spinlock = 10;
    ///     } else {
    ///         println!("try_lock failed");
    ///     }
    /// }).join().expect("thread::spawn failed");
    /// let guard = spinlock.lock();
    /// assert_eq!(*guard, 10);
    /// guard.unlock();
    /// ```
    #[track_caller]
    pub fn try_lock(&self) -> Result<SpinLockGuard<'_, T>, WouldBlock> {
        let guard = crate::interrupt::InterruptGuard::new();
        let acquired = !self.locked.fetch_or(true, Ordering::SeqCst);
        if acquired {
            Ok(SpinLockGuard {
                guard: Some(guard),
                caller: core::panic::Location::caller(),
                lock: self,
            })
        } else {
            Err(WouldBlock)
        }
    }

    /// Consumes this spinlock, returning the underlying data.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::sync::SpinLock;
    ///
    /// let spinlock = SpinLock::new(0);
    /// assert_eq!(spinlock.into_inner().unwrap(), 0);
    /// ```
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.data.into_inner()
    }
}

impl<T: Default> Default for SpinLock<T> {
    /// Creates a `SpinLock<T>`, with the `Default` value for T.
    fn default() -> SpinLock<T> {
        SpinLock::new(Default::default())
    }
}

impl<T: ?Sized> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

/// An implementation of a "scoped lock" of a spinlock. When this structure
/// is dropped (falls out of scope) without unlock, panic occurs.
///
/// The lock must be explicitly unlocked by [`unlock`] method.
///
/// The data protected by the mutex can be accessed through this guard.
///
/// This structure is created by the [`lock`] and [`try_lock`] methods on
/// [`SpinLock`].
///
/// [`lock`]: SpinLock::lock
/// [`try_lock`]: SpinLock::try_lock
/// [`unlock`]: Self::unlock
pub struct SpinLockGuard<'a, T: ?Sized + 'a> {
    caller: &'static core::panic::Location<'static>,
    lock: &'a SpinLock<T>,
    guard: Option<crate::interrupt::InterruptGuard>,
}

impl<T: ?Sized> !Send for SpinLockGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for SpinLockGuard<'_, T> {}

impl<T: ?Sized> SpinLockGuard<'_, T> {
    /// Releases the underlying [`SpinLock`].
    ///
    /// As the guard does **not** automatically release the lock on drop,
    /// the caller must explicitly invoke [`unlock`] to mark the lock
    /// as available again.
    ///
    /// # Example
    /// ```
    /// let lock = SpinLock::new(123);
    /// let guard = lock.lock();
    ///
    /// // Work with the locked data...
    ///
    /// // Explicitly release the lock.
    /// guard.unlock();
    /// ```
    pub fn unlock(mut self) {
        self.lock.locked.store(false, Ordering::SeqCst);
        self.guard.take();
        core::mem::forget(self);
    }
}

impl<T: ?Sized> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        panic!(
            "`.unlock()` must be explicitly called before dropping SpinLockGuard.
The lock is held at {:?}.",
            self.caller
        );
    }
}
