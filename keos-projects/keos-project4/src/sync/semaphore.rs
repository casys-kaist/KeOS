//! # Semaphore.
//!
//! A **semaphore** is a fundamental synchronization primitive used to regulate
//! concurrent access to a finite set of resources. It maintains an internal
//! count representing the number of available "permits." Each permit grants a
//! thread the right to access a shared resource. Semaphores are particularly
//! well-suited for controlling resource allocation and coordinating
//! thread execution in systems with bounded capacity constraints.
//!
//! Conceptually, a semaphore can be implemented using a combination of a
//! mutex for mutual exclusion and a condition variable to support blocking and
//! waking threads. This approach ensures thread-safe and efficient control over
//! the internal permit count.
//!
//! Semaphores are widely used in operating systems to solve classic concurrency
//! problems. One common example is the **producer-consumer** pattern, in which
//! multiple threads coordinate access to a bounded buffer. A semaphore ensures
//! that producers do not overfill the buffer and consumers do not read from an
//! empty buffer.
//!
//! Another critical use case is **event signaling**. A semaphore initialized
//! with zero permits can serve as a one-time or repeating signal, allowing one
//! thread to notify another when a particular event has occurred.
//!
//! ## `Semaphore` in KeOS
//!
//! In KeOS, the [`Semaphore`] abstraction provides a clean and safe interface
//! for controlling access to shared resources. [`Semaphore`] is combined with a
//! resource to protect. Threads can acquire a permit by calling
//! [`Semaphore::wait`], and release it either explicitly via
//! [`Semaphore::signal`] or implicitly using the [`SemaphorePermits`] RAII
//! guard. This design encourages robust and leak-free resource management, even
//! in the presence of early returns or panics.
//!
//! Key features of the `Semaphore` interface include:
//!
//! - [`Semaphore::wait()`]: Decrements the permit count if a permit is
//!   available. If no permits remain, the calling thread blocks until one
//!   becomes available.
//!
//! - [`Semaphore::signal()`]: Increments the permit count and wakes one blocked
//!   thread, if any. This allows threads to proceed once a resource has been
//!   released or an event has been signaled.
//!
//! - [`SemaphorePermits`]: An RAII-based wrapper that automatically calls
//!   `signal()` when dropped. This ensures that permits are always correctly
//!   released, even in the face of errors or control-flow disruptions.
//!
//! By integrating semaphores into the KeOS kernel, you will gain a flexible and
//! expressive tool for coordinating access to shared resources and implementing
//! complex thread synchronization patterns.
//!
//! #### Usage Example
//!
//! ```rust
//! let sema = Semaphore::new(3, state); // Allows up to 3 concurrent threads to the state
//!
//! // Acquire a permit (blocks if unavailable)
//! let permit = sema.wait();
//!
//! // Critical section (up to 3 threads can enter concurrently)
//! permit.work(); // Call a method defined on the `state`.
//!
//! // Permit is automatically released when `permit` goes out of scope.
//! // Otherwise, you can explicitly released it with `drop(permit)``
//! ```
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`Semaphore`]
//! - [`Semaphore::new`]
//! - [`Semaphore::wait`]
//! - [`Semaphore::signal`]
//!
//! By implementing the all synchorinzation primitives, your KeOS kernel now
//! ready to serve multi-threaded process in the next [`section`].
//!
//! [`wait`]: Semaphore::wait
//! [`signal`]: Semaphore::signal
//! [`SemaphorePermits`]: SemaphorePermits
//! [`Mutex`]: crate::sync::Mutex
//! [`ConditionVariable`]: crate::sync::ConditionVariable
//! [`section`]: crate::process

use core::ops::Deref;

use super::{condition_variable::ConditionVariable, mutex::Mutex};

/// Counting semaphore.
///
/// A semaphore maintains a set of permits and resource. Permits are used to
/// synchronize access to a shared resource. A semaphore differs from a mutex in
/// that it can allow more than one concurrent caller to access the shared
/// resource at a time.
pub struct Semaphore<T> {
    resource: T,
    // TODO: Add any member you need.
}

impl<T> Semaphore<T> {
    /// Creates a new semaphore initialized with a specified number of permits.
    ///
    /// # Arguments
    ///
    /// * `permits` - The initial number of available permits. Must be a
    ///   non-negative number.
    /// * `state` - A resource combined with this resource
    pub fn new(permits: usize, resource: T) -> Self {
        Self {
            resource,
            // TODO: Initialize the members you added.
        }
    }

    /// Waits until a permit becomes available and then acquires it.
    ///
    /// If no permits are available, this function will block the current thread
    /// until another thread calls `signal()` to release a permit.
    ///
    /// This method returns a [`SemaphorePermits`] RAII guard. When the guard is
    /// dropped, it will automatically release the acquired permit.
    pub fn wait(&self) -> SemaphorePermits<'_, T> {
        todo!()
    }

    /// Releases a permit back to the semaphore.
    ///
    /// This method increases the number of available permits by one, and if any
    /// threads are blocked in `wait()`, one will be woken up to acquire the
    /// newly released permit.
    ///
    /// Normally, you don’t call this directly except for signaling an event
    /// with a zero-initialized semaphore. Instead, it's automatically invoked
    /// when a [`SemaphorePermits`] guard is dropped.
    pub fn signal(&self) {
        todo!()
    }
}

/// An RAII implementation of a "scoped semaphore". When this structure
/// is dropped (falls out of scope), the semaphore will be signaled.
///
/// The data protected by the semaphore can be accessed through this guard via
/// its [`Deref`] implementations.
///
/// This structure is created by the [`wait`] method on [`Semaphore`].
///
/// [`wait`]: Semaphore::wait
pub struct SemaphorePermits<'a, T> {
    sema: &'a Semaphore<T>,
}

impl<T> Deref for SemaphorePermits<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.sema.resource
    }
}

impl<T> Drop for SemaphorePermits<'_, T> {
    fn drop(&mut self) {
        self.sema.signal()
    }
}
