//! SMP-supported spinlock.
//!
//! The implementing unicore spinlock uniprocessor is simple; it just requires
//! preventing thread preemption while holding a lock. By disabling preemption
//! of the lock-holding thread,  other threads cannot access shared resource as
//! they can't be scheduled.
//!
//! However, when it comes to multiprocessor, disabling preemption is not
//! sufficient; as multiple threads run concurrently in different cores, they
//! can access shared resource at the same time even when a core disable
//! preemption. Therefore, to acquire a lock on multi-processor, a processor 1)
//! polls a variable that represents a value is locked or not  2) set the
//! variable when a thread holds the `lock`, and 3) unset the variable when the
//! thread `unlock`.
//!
//! The step 1 and 2 must be executed ATOMICALLY with the atomic
//! read-modify-write instructions of the CPU.
//!
//! This module introduce the support of the SMP-supported spinlock in KeOS.

pub use abyss::spinlock::WouldBlock;

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
///         let mut data = data.lock().unwrap();
///         *data += 1;
///         // the lock must be "explicitly" unlocked.
///         data.unlock();
///     });
/// }
/// ```
pub use abyss::spinlock::SpinLock;
pub use abyss::spinlock::SpinLockGuard;
