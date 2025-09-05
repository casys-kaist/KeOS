//! # Condition Variable.
//!
//! A **Condition Variable** allows a thread to efficiently block until a
//! certain condition is met, without consuming CPU cycles. It is always used
//! in conjunction with a [`Mutex`] that guards access to shared data. It is
//! generally used when a thread needs to *wait for a specific state*
//! in shared data, and another thread will *notify* it when that state changes.
//!
//! Condition variables enable complex synchronization patterns such as thread
//! coordination, resource availability signaling, or implementing blocking
//! queues.
//!
//! ## `ConditionVariable` in KeOS
//! Condition variable must work with the shared [`Mutex`]. To enforce this,
//! KeOS's [`ConditionVariable`] api takes either [`Mutex`] or [`MutexGuard`] as
//! an argument. This enforces that the apis are called with a mutex, but does
//! not fully ensure that the mutex is the associated one.
//!
//! The [`ConditionVariable::wait_while`] method automatically checks the
//! predicate, blocks the current thread if the condition is true, and re-checks
//! it upon wakeup. The method takes care of locking, checking the condition,
//! blocking, and waking up:
//!
//! ```rust
//! let guard = condvar.wait_while(&mutex, |state| state.is_empty());
//! ```
//!
//! There are two signaling methods that takes the [`MutexGuard`]:
//! - [`ConditionVariable::signal`] wakes **one** waiting thread and
//! - [`ConditionVariable::broadcast`] wakes **all** waiting threads.
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`ConditionVariable::wait_while`]
//! - [`ConditionVariable::signal`]
//! - [`ConditionVariable::broadcast`]
//!
//! After implement the functionalities, move on to the next [`section`].
//!
//! [`Mutex`]: crate::sync::Mutex
//! [`section`]: crate::sync::semaphore

use super::mutex::{Mutex, MutexGuard};
use alloc::collections::vec_deque::VecDeque;
use keos::{sync::SpinLock, thread::ParkHandle};

/// A Condition Variable
///
/// Condition variables represent the ability to block a thread such that it
/// consumes no CPU time while waiting for an event to occur. Condition
/// variables are typically associated with a boolean predicate (a condition)
/// and a mutex. The predicate is always verified inside of the mutex before
/// determining that a thread must block.
///
/// Functions in this module will block the current **thread** of execution.
/// Note that any attempt to use multiple mutexes on the same condition
/// variable may result in a runtime panic.
#[derive(Default)]
pub struct ConditionVariable {
    waiters: SpinLock<VecDeque<ParkHandle>>,
}

impl ConditionVariable {
    /// Creates a new condition variable which is ready to be waited on and
    /// signaled.
    pub fn new() -> Self {
        Self {
            waiters: SpinLock::new(VecDeque::new()),
        }
    }

    /// Blocks the current thread while `predicate` returns `true`.
    ///
    /// This function takes reference of a [`Mutex`] and checks the
    /// predicate. If it returns `true`, the thread is blocked and the mutex is
    /// temporarily released. When the thread is signaled and wakes up, it
    /// reacquires the mutex and re-evaluates the predicate. This loop continues
    /// until the predicate returns `false`.
    ///
    /// # Example
    /// ```rust
    /// let guard = condvar.wait_while(&mutex, |state| state.count == 0);
    /// ```
    ///
    /// There is **no need to check the predicate before calling** `wait_while`.
    /// It performs the entire check-and-sleep logic internally.
    pub fn wait_while<'a, T>(
        &self,
        mutex: &'a Mutex<T>,
        predicate: impl Fn(&mut T) -> bool,
    ) -> MutexGuard<'a, T> {
        todo!()
    }

    /// Wakes up one blocked thread on this condvar.
    ///
    /// If there is a blocked thread on this condition variable, then it will
    /// be woken up from its call to [`wait_while`]. Calls to `signal` are not
    /// buffered in any way.
    ///
    /// To wake up all threads, see [`broadcast`].
    ///
    /// [`broadcast`]: ConditionVariable::broadcast
    /// [`wait_while`]: ConditionVariable::wait_while
    pub fn signal<'a, T>(&self, guard: MutexGuard<'a, T>) {
        todo!()
    }

    /// Wakes up all blocked threads on this condvar.
    ///
    /// This method will ensure that any current waiters on the condition
    /// variable are awoken. Calls to `broadcast()` are not buffered in any
    /// way.
    ///
    /// To wake up only one thread, see [`signal`].
    ///
    /// [`signal`]: ConditionVariable::signal
    pub fn broadcast<'a, T>(&self, guard: MutexGuard<'a, T>) {
        todo!()
    }
}
