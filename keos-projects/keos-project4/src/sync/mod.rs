//! # Synchronization Primitives.
//!
//! An operating system kernel must coordinate access to shared resources among
//! multiple threads of execution. This coordination is vital to ensure data
//! consistency, prevent race conditions, and maintain overall system stability
//! and performance.
//!
//! In earlier projects, KeOS employed a single synchronization primitive: the
//! [`SpinLock`]. While [`SpinLock`] provides correct mutual exclusion by
//! repeatedly checking for lock availability (i.e., "spinning"), it is
//! inefficient under high contention, as it wastes CPU cycles by actively
//! polling instead of sleeping.
//!
//! In this project, you will address these limitations by implementing
//! additional synchronization primitives that offer more efficient and
//! expressive mechanisms for managing concurrency. These primitives are
//! essential tools for building correct and performant multithreaded
//! systems, particularly in contexts where threads may need to sleep,
//! coordinate, or share limited resources.
//!
//! The synchronization primitives you will implement include:
//!
//! - [`Mutex`]: A mutual exclusion primitive that ensures only one thread can
//!   access a critical section at a time. Unlike a spinlock, a mutex puts the
//!   thread to sleep if the lock is unavailable, avoiding busy-waiting and
//!   reducing CPU usage.
//!
//! - [`ConditionVariable`]: A coordination mechanism that allows threads to
//!   sleep until a particular condition becomes true. It enables efficient
//!   producer-consumer style synchronization and is often used in conjunction
//!   with a mutex.
//!
//! - [`Semaphore`]: A counting synchronization primitive that controls access
//!   to a shared resource by maintaining a counter. It allows a fixed number of
//!   threads to access the resource concurrently and can also be used to
//!   implement other synchronization patterns.
//!
//! Together, these primitives provide a flexible foundation for managing
//! concurrency in a robust and scalable kernel. They will enable you to
//! implement more sophisticated system services, such as blocking system calls
//! and multithreaded applications.
//!
//! Different synchronization primitives are suited to different concurrency
//! patterns. The table below summarizes their key characteristics:
//!
//! | Primitive             | Blocks Thread? | Fair?    | Typical Use Case                                 |
//! |-----------------------|----------------|----------|--------------------------------------------------|
//! | [`SpinLock`]          | No (busy wait) | No       | Short, uncontended critical sections in the kernel |
//! | [`Mutex`]             | Yes            | Yes      | Exclusive access to shared data                 |
//! | [`ConditionVariable`] | Yes            | Yes      | Waiting for a condition to become true          |
//! | [`Semaphore`]         | Yes            | Depends  | Limiting access to a bounded resource            |
//!
//! - **SpinLock** spins in a loop until the lock becomes available. This is
//!   suitable for extremely short operations in low-contention paths.
//! - **Mutex** puts the thread to sleep when the lock is unavailable, making it
//!   ideal for when exclusive access is needed and blocking is acceptable
//! - **ConditionVariable** works in tandem with a mutex to block threads until
//!   a predicate becomes false, then wake them up. Meaning that it fits when a
//!   thread must wait for a condition while coordinating with a [`Mutex`].
//! - **Semaphore** tracks a count of available permits and is often used to
//!   control access to a pool of resources or to implement thread joins. It can
//!   be used when multiple threads can proceed concurrently, up to a fixed
//!   limit.
//!
//! ## Implementation Orders
//! 1. [`mutex`]
//! 2. [`condition_variable`]
//! 3. [`semaphore`]
//!
//! [`mutex`]: self::mutex
//! [`condition_variable`]: self::condition_variable
//! [`semaphore`]: self::semaphore
//! [`SpinLock`]: keos::sync::SpinLock
//! [`Mutex`]: crate::sync::mutex::Mutex
//! [`ConditionVariable`]: crate::sync::condition_variable::ConditionVariable
//! [`Semaphore`]: crate::sync::semaphore::Semaphore

pub mod condition_variable;
pub mod mutex;
pub mod semaphore;

pub use condition_variable::*;
pub use mutex::*;
pub use semaphore::*;
