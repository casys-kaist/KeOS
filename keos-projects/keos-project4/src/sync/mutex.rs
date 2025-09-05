//! # Mutex.
//!
//! Mutex is a synchronization primitive that allows **only one thread at a
//! time** to access a critical section of code, protecting shared resources
//! such as memory, files, or device state from concurrent modification.
//! Unlike the spin lock, it **blocks** threads trying to acquire it if another
//! thread already holds the lock.
//!
//! The [`Mutex`] maintains the list of threads sleeping on the mutex.
//! When unlocking, the kernel **wakes up** one of the waiting threads.
//! In KeOS, you can sleep the current thread by calling the
//! [`Current::park_with`]. This function takes a closure to run before falling
//! in sleep with a argument [`ParkHandle`], which is the handle to wake up the
//! sleeping thread.
//!
//! Although mutex and spin lock provides similar synchronization guarantees,
//! they are used in different circumstances. The following table compares the
//! spin lock and mutex.
//!
//! |                | SpinLock               | Mutex                    |
//! |----------------|-------------------------|--------------------------|
//! | Waiting thread | Spins (busy-waits)       | Sleeps                   |
//! | CPU usage      | High (wastes CPU cycles) | Low (no busy waiting)     |
//! | Overhead       | Low (fast if uncontended)| Higher (due to sleep/wake)|
//!
//! These characterists lead that the spin lock is suitable when critical
//! sections are extremely short and contention is rare, because spinning wastes
//! CPU cycles. On the other side, the mutex is better for longer critical
//! sections or when a lock may be held for a non-trivial amount of time, as
//! sleeping threads do not waste CPU.
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`Mutex`]
//! - [`Mutex::new`]
//! - [`Mutex::lock`]
//! - [`MutexGuard::unlock`]
//!
//! After implement the functionalities, move on to the next [`section`].
//!
//! [`section`]: crate::sync::condition_variable
//! [`Current::park_with`]: keos::thread::Current::park_with

use alloc::collections::vec_deque::VecDeque;
use core::ops::{Deref, DerefMut};
use keos::{
    sync::{SpinLock, SpinLockGuard, WouldBlock},
    thread::ParkHandle,
};

/// A mutual exclusion primitive useful for protecting shared data
///
/// This mutex will block threads waiting for the lock to become available.
/// The mutex can be created via a [`new`] constructor. Each spinlock has a
/// type parameter which represents the data that it is protecting. The data can
/// only be accessed through the guards returned from [`lock`] and
/// [`try_lock`], which guarantees that the data is only ever accessed when the
/// mutex is locked.
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
/// use keos::sync::Mutex;
/// use keos::thread;
///
/// const N: usize = 10;
///
/// // Spawn a few threads to increment a shared variable (non-atomically), and
/// // let the main thread know once all increments are done.
/// //
/// // Here we're using an Arc to share memory among threads, and the data inside
/// // the Arc is protected with a mutex.
/// let data = Arc::new(Mutex::new(0));
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
///         let mut data = data.lock().unwrap();
///         *data += 1;
///         // the lock must be "explicitly" unlocked.
///         data.unlock();
///     });
/// }
/// ```
pub struct Mutex<T> {
    // TODO: Define any member you need.
    t: SpinLock<T>,
    waiters: SpinLock<VecDeque<ParkHandle>>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::sync::Mutex;
    ///
    /// let mutex = Mutex::new(0);
    /// ```
    #[inline]
    pub const fn new(t: T) -> Mutex<T> {
        Mutex {
            // TODO: Initialize the members you added.
            t: SpinLock::new(t),
            waiters: SpinLock::new(VecDeque::new()),
        }
    }
}

impl<T> Mutex<T> {
    /// Acquires a mutex, blocking the current thread until it is able to do
    /// so.
    ///
    /// This function will block the local thread until it is available to
    /// acquire the mutex. Upon returning, the thread is the only thread
    /// with the lock held. An guard is returned to allow scoped unlock
    /// of the lock. When the guard goes out of scope, the mutex will be
    /// unlocked.
    ///
    /// The exact behavior on locking a mutex in the thread which already
    /// holds the lock is left unspecified. However, this function will not
    /// return on the second call (it might panic or deadlock, for example).
    ///
    /// # Examples
    ///
    /// ```
    /// use alloc::sync::Arc;
    /// use keos::sync::Mutex;
    /// use keos::thread;
    ///
    /// let mutex = Arc::new(Mutex::new(0));
    /// let c_mutex = Arc::clone(&spinlock);
    ///
    /// thread::spawn(move || {
    ///     *c_mutex.lock().unwrap() = 10;
    /// }).join().expect("thread::spawn failed");
    /// assert_eq!(*mutex.lock().unwrap(), 10);
    /// ```
    pub fn lock(&self) -> MutexGuard<'_, T> {
        todo!()
    }
    /// Attempts to acquire this lock.
    ///
    /// If the lock could not be acquired at this time, then [`Err`] is
    /// returned. Otherwise, an guard is returned.
    ///
    /// This function does not block.
    ///
    /// # Errors
    ///
    /// If the mutex could not be acquired because it is already locked, then
    /// this call will return the [`WouldBlock`] error.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::sync::Mutex;
    /// use alloc::sync::Arc;
    /// use keos::thread;
    ///
    /// let mutex = Arc::new(Mutex::new(0));
    /// let c_mutex = Arc::clone(&spinlock);
    ///
    /// thread::spawn(move || {
    ///     let mut lock = c_mutex.try_lock();
    ///     if let Ok(ref mut mutex) = lock {
    ///         **mutex = 10;
    ///     } else {
    ///         println!("try_lock failed");
    ///     }
    /// }).join().expect("thread::spawn failed");
    /// assert_eq!(*mutex.lock().unwrap(), 10);
    /// ```
    pub fn try_lock(&self) -> Result<MutexGuard<'_, T>, WouldBlock> {
        if let Ok(guard) = self.t.try_lock() {
            Ok(MutexGuard {
                guard: Some(guard),
                lock: self,
            })
        } else {
            Err(WouldBlock)
        }
    }

    /// Consumes this mutex, returning the underlying data.
    ///
    /// # Examples
    ///
    /// ```
    /// use keos::sync::Mutex;
    ///
    /// let mutex = Mutex::new(0);
    /// assert_eq!(mutex.into_inner().unwrap(), 0);
    /// ```
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.t.into_inner()
    }
}

impl<T: Default> Default for Mutex<T> {
    /// Creates a `Mutex<T>`, with the `Default` value for T.
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.guard.as_ref().unwrap()
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.guard.as_mut().unwrap()
    }
}

/// An implementation of a "scoped lock" of a mutex. When this structure
/// is dropped (falls out of scope) without unlocking, the panic occurs.
///
/// The lock must be explicitly unlocked by [`unlock`] method.
///
/// The data protected by the mutex can be accessed through this guard.
///
/// This structure is created by the [`lock`] and [`try_lock`] methods on
/// [`Mutex`].
///
/// [`lock`]: Mutex::lock
/// [`try_lock`]: Mutex::try_lock
/// [`unlock`]: MutexGuard::unlock
pub struct MutexGuard<'a, T: 'a> {
    guard: Option<SpinLockGuard<'a, T>>,
    lock: &'a Mutex<T>,
}

impl<T> !Send for MutexGuard<'_, T> {}
unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}

impl<T> MutexGuard<'_, T> {
    /// Releases the underlying [`Mutex`].
    ///
    /// As the guard does **not** automatically release the lock on drop,
    /// the caller must explicitly invoke [`unlock`] to mark the lock
    /// as available again.
    ///
    /// # Example
    /// ```
    /// let lock = Mutex::new(123);
    /// let guard = lock.lock();
    ///
    /// // Work with the locked data...
    ///
    /// // Explicitly release the lock.
    /// guard.unlock();
    /// ```
    /// [`unlock`]: MutexGuard::unlock
    pub fn unlock(mut self) {
        todo!()
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        panic!("`.unlock()` must be explicitly called for MutexGuard.");
    }
}
