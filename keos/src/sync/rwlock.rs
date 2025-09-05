//! RwLock implementations.

use abyss::spinlock::{SpinLock, SpinLockGuard};
use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

/// A reader-writer lock
///
/// This type of lock allows a number of readers or at most one writer at any
/// point in time. The write portion of this lock typically allows modification
/// of the underlying data (exclusive access) and the read portion of this lock
/// typically allows for read-only access (shared access).
///
/// In comparison, a [`Mutex`] does not distinguish between readers or writers
/// that acquire the lock, therefore blocking any threads waiting for the lock
/// to become available. An `RwLock` will allow any number of readers to acquire
/// the lock as long as a writer is not holding the lock.
///
/// The priority policy of the lock is dependent on the underlying operating
/// system's implementation, and this type does not guarantee that any
/// particular policy will be used.
///
/// The type parameter `T` represents the data that this lock protects. It is
/// required that `T` satisfies [`Send`] to be shared across threads and
/// [`Sync`] to allow concurrent access through readers. The RAII guards
/// returned from the locking methods implement [`Deref`] (and [`DerefMut`]
/// for the `write` methods) to allow access to the content of the lock.
///
/// [`Mutex`]: struct.Mutex.html
pub struct RwLock<T>
where
    T: ?Sized + Send,
{
    // state:
    // Upper 2bit represent the lock state.
    // 0: Nobody try to get lock for writing.
    // 1: Writer is waiting.
    // 2: Writer holds the lock.
    state: AtomicUsize,
    owner: SpinLock<Option<(u64, &'static core::panic::Location<'static>)>>,
    data: UnsafeCell<T>,
}

const STATE_MASK: usize = 0b1 << (usize::BITS - 2);
const STATE_WRITER_LOCKED: usize = 0b1 << (usize::BITS - 2);

#[inline]
fn is_write_locked(b: usize) -> bool {
    b & STATE_MASK == STATE_WRITER_LOCKED
}

/// RAII structure used to release the exclusive write access of a lock when
/// dropped.
///
/// This structure is created by the [`write`] and [`try_write`] methods
/// on [`RwLock`].
///
/// [`write`]: struct.RwLock.html#method.write
/// [`try_write`]: struct.RwLock.html#method.try_write
/// [`RwLock`]: struct.RwLock.html
pub struct RwLockWriteGuard<'a, T>
where
    T: ?Sized + Send,
    T: 'a,
{
    lock: &'a RwLock<T>,
    data: &'a mut T,
}

/// RAII structure used to release the shared read access of a lock when
/// dropped.
///
/// This structure is created by the [`read`] and [`try_read`] methods on
/// [`RwLock`].
///
/// [`read`]: struct.RwLock.html#method.read
/// [`try_read`]: struct.RwLock.html#method.try_read
/// [`RwLock`]: struct.RwLock.html
pub struct RwLockReadGuard<'a, T>
where
    T: ?Sized + Send,
    T: 'a,
{
    lock: &'a RwLock<T>,
    data: &'a T,
}

impl<'a, T> RwLockReadGuard<'a, T>
where
    T: ?Sized + Send,
    T: 'a,
{
    /// Upgrade the `RwLockReadGuard`` into `RwLockWriteGuard`.
    #[track_caller]
    pub fn upgrade(self) -> RwLockWriteGuard<'a, T> {
        let this = core::mem::ManuallyDrop::new(self);
        let lock = unsafe { core::ptr::read(&this.lock) };
        loop {
            let mut guard = lock.owner.lock();
            if lock
                .state
                .compare_exchange(1, STATE_WRITER_LOCKED, Ordering::Acquire, Ordering::Acquire)
                .is_ok()
            {
                *guard = Some((
                    crate::thread::Current::get_tid(),
                    core::panic::Location::caller(),
                ));
                guard.unlock();

                break RwLockWriteGuard {
                    lock,
                    data: unsafe { &mut *lock.data.get() },
                };
            }
            guard.unlock();
        }
    }
}

impl<'a, T> RwLockWriteGuard<'a, T>
where
    T: ?Sized + Send,
    T: 'a,
{
    /// Downgrade the `RwLockWriteGuard` into `RwLockReadGuard`.
    pub fn downgrade(self) -> RwLockReadGuard<'a, T> {
        let this = core::mem::ManuallyDrop::new(self);
        let lock = unsafe { core::ptr::read(&this.lock) };
        assert!(
            lock.state
                .compare_exchange(STATE_WRITER_LOCKED, 1, Ordering::Acquire, Ordering::Acquire)
                .is_ok()
        );
        RwLockReadGuard {
            lock,
            data: unsafe { &*lock.data.get() },
        }
    }
}

impl<T> RwLock<T>
where
    T: Send,
{
    /// Creates a new instance of an `RwLock<T>` which is unlocked.
    pub const fn new(data: T) -> RwLock<T> {
        RwLock {
            state: AtomicUsize::new(0),
            owner: SpinLock::new(None),
            data: UnsafeCell::new(data),
        }
    }

    #[inline]
    fn validate_state(
        &self,
        owner: SpinLockGuard<Option<(u64, &'static core::panic::Location<'static>)>>,
    ) {
        {
            let owner = owner.expect("RwLock is in unexpected state.");
            if owner.0 == crate::thread::Current::get_tid() {
                panic!(
                    "Try to acquiring ReadGuard on the thread holding the WriteGuard acquired on {:?}.",
                    owner.1
                );
            }
        }
        owner.unlock();
    }

    #[inline]
    fn read_lock(&self) {
        loop {
            let guard = self.owner.lock();
            let prev = self.state.load(Ordering::Relaxed);
            if is_write_locked(prev) {
                self.validate_state(guard);
                core::hint::spin_loop();
            } else if self
                .state
                .compare_exchange(prev, prev + 1, Ordering::Acquire, Ordering::Acquire)
                .is_ok()
            {
                guard.unlock();
                break;
            } else {
                guard.unlock();
            }
        }
    }

    /// Locks this rwlock with shared read access, blocking the current thread
    /// until it can be acquired.
    ///
    /// The call
    /// ing thread will be blocked until there are no more writers which
    /// hold the lock. There may be other readers currently inside the lock when
    /// this method returns. This method does not provide any guarantees with
    /// respect to the ordering of whether contentious readers or writers will
    /// acquire the lock first.
    ///
    /// Returns an RAII guard which will release this thread's shared access
    /// once it is dropped.
    #[inline]
    #[track_caller]
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        if let Ok(guard) = self.try_read() {
            guard
        } else {
            self.read_lock();
            RwLockReadGuard {
                lock: self,
                data: unsafe { &*self.data.get() },
            }
        }
    }

    /// Attempts to acquire this rwlock with shared read access.
    ///
    /// If the access could not be granted at this time, then `Err` is returned.
    /// Otherwise, an RAII guard is returned which will release the shared
    /// access when it is dropped.
    ///
    /// This function does not block.
    ///
    /// This function does not provide any guarantees with respect to the
    /// ordering of whether contentious readers or writers will acquire the
    /// lock first.
    #[inline]
    #[track_caller]
    pub fn try_read(&self) -> Result<RwLockReadGuard<'_, T>, crate::spinlock::WouldBlock> {
        loop {
            let guard = self.owner.lock();
            let prev = self.state.load(Ordering::Relaxed);
            if is_write_locked(prev) {
                self.validate_state(guard);
                break Err(crate::spinlock::WouldBlock);
            } else if self
                .state
                .compare_exchange(prev, prev + 1, Ordering::Acquire, Ordering::Acquire)
                .is_ok()
            {
                guard.unlock();
                break Ok(RwLockReadGuard {
                    lock: self,
                    data: unsafe { &*self.data.get() },
                });
            }
            guard.unlock();
        }
    }

    #[inline]
    fn write_lock(&self) {
        loop {
            let prev = self.state.load(Ordering::Relaxed);
            if prev > 0 {
                core::hint::spin_loop();
            } else if self
                .state
                .compare_exchange(0, STATE_WRITER_LOCKED, Ordering::Acquire, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Locks this rwlock with exclusive write access, blocking the current
    /// thread until it can be acquired.
    ///
    /// This function will not return while other writers or other readers
    /// currently have access to the lock.
    ///
    /// Returns an RAII guard which will drop the write access of this rwlock
    /// when dropped.
    #[inline]
    #[track_caller]
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        if let Ok(guard) = self.try_write() {
            guard
        } else {
            let mut guard = self.owner.lock();
            self.write_lock();
            *guard = Some((
                crate::thread::Current::get_tid(),
                core::panic::Location::caller(),
            ));
            guard.unlock();
            RwLockWriteGuard {
                lock: self,
                data: unsafe { &mut *self.data.get() },
            }
        }
    }

    /// Attempts to lock this rwlock with exclusive write access.
    ///
    /// If the lock could not be acquired at this time, then `Err` is returned.
    /// Otherwise, an RAII guard is returned which will release the lock when
    /// it is dropped.
    ///
    /// This function does not block.
    ///
    /// This function does not provide any guarantees with respect to the
    /// ordering of whether contentious readers or writers will acquire the
    /// lock first.
    #[track_caller]
    pub fn try_write(&self) -> Result<RwLockWriteGuard<'_, T>, crate::spinlock::WouldBlock> {
        loop {
            let mut guard = self.owner.lock();
            let prev = self.state.load(Ordering::Relaxed);
            if prev > 0 {
                guard.unlock();
                break Err(crate::spinlock::WouldBlock);
            } else if self
                .state
                .compare_exchange(
                    prev,
                    prev | STATE_WRITER_LOCKED,
                    Ordering::Acquire,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                *guard = Some((
                    crate::thread::Current::get_tid(),
                    core::panic::Location::caller(),
                ));
                guard.unlock();

                break Ok(RwLockWriteGuard {
                    lock: self,
                    data: unsafe { &mut *self.data.get() },
                });
            }
            guard.unlock();
        }
    }
    /// This steals the ownership even if the value is locked. Racy.
    ///
    /// # Safety
    /// This is unsafe.
    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn steal(&self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }

    /// Consumes this RwLock, returning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

unsafe impl<T> Sync for RwLock<T> where T: ?Sized + Send {}
unsafe impl<T> Send for RwLock<T> where T: ?Sized + Send {}

impl<T: Send> core::fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("RwLock")
            .field("state", &self.state.load(Ordering::SeqCst))
            .finish()
    }
}

impl<'a, T> Deref for RwLockReadGuard<'a, T>
where
    T: ?Sized + Send,
{
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T> Deref for RwLockWriteGuard<'a, T>
where
    T: ?Sized + Send,
{
    type Target = T;
    fn deref(&self) -> &T {
        &*self.data
    }
}

impl<'a, T> DerefMut for RwLockWriteGuard<'a, T>
where
    T: ?Sized + Send,
{
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.data
    }
}

impl<'a, T> Drop for RwLockReadGuard<'a, T>
where
    T: ?Sized + Send,
{
    #[track_caller]
    fn drop(&mut self) {
        debug_assert_eq!(self.lock.state.load(Ordering::Acquire) & STATE_MASK, 0);
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

impl<'a, T> Drop for RwLockWriteGuard<'a, T>
where
    T: ?Sized + Send,
{
    #[track_caller]
    fn drop(&mut self) {
        debug_assert_eq!(
            self.lock.state.load(Ordering::Acquire) & STATE_MASK,
            STATE_WRITER_LOCKED
        );
        self.lock
            .state
            .fetch_and(!STATE_WRITER_LOCKED, Ordering::Release);
    }
}
