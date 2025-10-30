//! # Multithreaded Process
//!
//! Modern applications increasingly rely on **multithreading** to achieve
//! responsiveness, scalability, and efficient utilization of multicore
//! processors. By enabling a single process to run multiple threads
//! concurrently, the system can perform I/O and computation in parallel, reduce
//! latency, and better respond to user interactions or real-time events.
//!
//! To support such workloads, the operating system must provide robust
//! mechanisms for creating, scheduling, and synchronizing threads within a
//! single process. This includes managing shared memory access, ensuring
//! thread-safe operations, and allowing coordination through synchronization
//! primitives such as mutexes, condition variables, and semaphores.
//!
//! In KeOS, extending the process abstraction to support multithreading is a
//! critical step toward building a realistic and capable system. This
//! enhances your knowledge about how the multithreading, the foundation for
//! concurrent programming models, works in real-world operating system.
//!
//! ## Multithreading in KeOS
//!
//! Previously, a process in KeOS consisted of a single thread. Your goal is
//! extending the process to run **multiple concurrent threads**, each with its
//! own execution context but sharing the same address space and resources.
//! Each thread maintains its own register states while sharing the same states.
//!
//! In earlier projects, each `Process` owned its own [`FileStruct`] and
//! [`MmStruct`]. However, on the multi-threaded model, these components are
//! **shared across all threads** of a process. That is, they become **shared
//! resources**, requiring proper synchronization. To support shared and mutable
//! access, these resources are wrapped inside an `Arc<Mutex<_>>`.
//! - [`Arc`] provides shared ownership with reference counting.
//! - [`Mutex`] ensures exclusive access to mutable state.
//!
//! This allows multiple threads to safely access and modify shared structures
//! like file tables and virtual memory mappings.
//!
//! #### Thread Life Cycle
//!
//! KeOS supports a lightweight threading model within a single process,
//! enabling multiple threads to execute concurrently while sharing the same
//! address space. The life cycle of a thread is managed through four key system
//! calls:
//!
//! - [`thread_create`]: Creates a new thread within the same process, executing
//!   a given function on a user-supplied stack.
//! - [`thread_join`]: Waits for a specified thread to terminate and retrieves
//!   its return value.
//! - [`exit`]: Terminates the calling thread without affecting other threads in
//!   the same process.
//! - [`exit_group`]: Terminates all threads within the process simultaneously.
//!
//! When creating a new thread via [`thread_create`], the user must provide a
//! pointer to a valid, writable memory region that will serve as the new
//! thread’s stack. This approach mirrors Linux's `clone()` system call and
//! gives userspace full control over stack allocation and reuse. The kernel
//! validates that the provided stack lies within a properly mapped and writable
//! memory region to ensure memory safety.
//!
//! Threads can be terminated individually using the [`exit`] system call,
//! which affects only the calling thread. Other threads in the same process
//! continue executing. To coordinate with thread termination, a thread may
//! invoke [`thread_join`], which blocks until the target thread exits and
//! returns its result. This can be implemented using a [`Semaphore`]
//! initialized with zero permits, where the exiting thread signals completion
//! by releasing a permit.
//!
//! In contrast, [`exit_group`] is used when the entire process must be
//! terminated, bringing down all associated threads by calling
//! [`thread::kill_by_tid`]. This is necessary in scenarios such as a fatal
//! error in the main thread, unhandled signals, or explicit process termination
//! by the application. Unlike [`exit`], which only marks the calling thread for
//! termination, [`exit_group`] ensures that all threads in the process are
//! promptly and safely terminated, and that the process is cleaned up
//! consistently. This behavior aligns with the semantics of multi-threaded
//! processes in modern operating systems and prevents resource leaks or partial
//! process shutdowns.
//!
//! Together, these mechanisms provide a simple yet robust model for managing
//! thread life cycles in KeOS, balancing fine-grained control with process-wide
//! coordination.
//!
//! ## Implementation Requirements
//! You need to implement the followings:
//! - [`Thread`]
//! - [`Thread::from_file_mm_struct`]
//! - [`Thread::with_file_struct_mut`]
//! - [`Thread::with_mm_struct_mut`]
//! - [`Thread::thread_create`]
//! - [`Thread::exit`]
//! - [`Thread::thread_join`]
//! - [`Thread::exit_group`]
//!
//! By implementing this section, you can move on to the next [`section`] with
//! the final form of execution model that widely used in modern OSes:
//! ```text
//! +========= Process =========+
//! | Shared States:            |
//! |  - MmStruct               |
//! |  - FileStruct             |
//! |                           |
//! | Threads:                  |
//! |  +----- Thread 1 -----+   |
//! |  |  - Register State  |   |
//! |  |  - User Stack      |   |
//! |  +--------------------+   |
//! |           ...             |
//! |  +----- Thread N -----+   |
//! |  |  - Register State  |   |
//! |  |  - User Stack      |   |
//! |  +--------------------+   |
//! +===========================+
//! ```
//!
//! [`exit`]: Thread::exit
//! [`thread_create`]: Thread::thread_create
//! [`thread_join`]: Thread::thread_join
//! [`exit_group`]: Thread::exit_group
//! [`Arc`]: <https://doc.rust-lang.org/beta/alloc/sync/struct.Arc.html>
//! [`section`]: crate::round_robin
//! [`thread::kill_by_tid`]: keos::thread::kill_by_tid
//! [`Mutex`]: crate::sync::Mutex
//! [`Semaphore`]: crate::sync::semaphore

use alloc::{boxed::Box, string::String};
use keos::{KernelError, addressing::Pa, syscall::Registers, thread::ThreadBuilder};
use keos_project1::{file_struct::FileStruct, syscall::SyscallAbi};
use keos_project2::mm_struct::MmStruct;
use keos_project3::lazy_pager::LazyPager;

/// A thread state of project 4, which contains file and memory state.
pub struct Thread {
    pub tid: u64,
    pub page_table_pa: Pa,
    // TODO: Add and fix any member you need.
    pub file_struct: FileStruct,
    pub mm_struct: MmStruct<LazyPager>,
}

impl Default for Thread {
    fn default() -> Self {
        Self::from_file_mm_struct(FileStruct::new(), MmStruct::new(), 0)
    }
}

impl Thread {
    /// Create a thread with given [`MmStruct`].
    pub fn from_mm_struct(mm_struct: MmStruct<LazyPager>, tid: u64) -> Self {
        Self::from_file_mm_struct(FileStruct::new(), mm_struct, tid)
    }

    /// Create a thread with given [`MmStruct`] and [`FileStruct`].
    pub fn from_file_mm_struct(
        file_struct: FileStruct,
        mm_struct: MmStruct<LazyPager>,
        tid: u64,
    ) -> Self {
        let page_table_pa = mm_struct.page_table.pa();

        // TODO: Initialize any member you need.

        Self {
            // TODO: Add and fix any member you need.
            tid,
            page_table_pa,
            mm_struct,
            file_struct,
        }
    }

    /// Executes a closure with mutable access to the underlying file struct
    /// ([`FileStruct`]).
    ///
    /// This method provides a way to access and mutate the file struct
    /// associated with the current thread. It accepts a closure `f` that
    /// receives a mutable reference to the `FileStruct` and an
    /// additional argument of type `Args`.
    pub fn with_file_struct_mut<Args, R>(
        &self,
        f: impl FnOnce(&mut FileStruct, Args) -> R,
        args: Args,
    ) -> R {
        f(todo!(), args)
    }

    /// Executes a closure with mutable access to the underlying memory struct
    /// ([`MmStruct`]).
    ///
    /// This method provides a way to access and mutate the memory struct
    /// associated with the current thread. It accepts a closure `f` that
    /// receives a mutable reference to the `MmStruct<LazyPager>` and an
    /// additional argument of type `Args`.
    pub fn with_mm_struct_mut<Args, R>(
        &self,
        f: impl FnOnce(&mut MmStruct<LazyPager>, Args) -> R,
        args: Args,
    ) -> R {
        f(todo!(), args)
    }

    /// Executes a closure with mutable access to the underlying file struct
    /// ([`FileStruct`]) and memory struct ([`MmStruct`]).
    ///
    /// This method provides a way to access and mutate the file struct
    /// associated with the current thread. It accepts a closure `f` that
    /// receives a mutable reference to the `FileStruct` and an
    /// additional argument of type `Args`.
    pub fn with_file_mm_struct_mut<Args, R>(
        &self,
        f: impl FnOnce(&mut FileStruct, &mut MmStruct<LazyPager>, Args) -> R,
        args: Args,
    ) -> R {
        self.with_mm_struct_mut(
            |mm, args| self.with_file_struct_mut(|fs, args| f(fs, mm, args), args),
            args,
        )
    }

    /// Exit the current thread.
    ///
    /// This function terminates the calling thread, returning the provided
    /// exit code to any thread that `join`s on it.
    ///
    /// # Syscall API
    /// ```c
    /// void exit(int status);
    /// ```
    /// - `status`: The exit code returned to a joining thread.
    ///
    /// # Behavior
    /// - Wakes up any thread waiting via `thread_join`.
    /// - Cleans up thread-local resources.
    pub fn exit(&self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Create a new thread in the current process.
    ///
    /// This function creates a new thread that begins execution at the given
    /// entry point with the specified argument.
    ///
    /// # Syscall API
    /// ```c
    /// int thread_create(char *name, void *stack, void *(*start_routine)(void *), void *arg);
    /// ```
    /// - `name`: Name of the thread.
    /// - `stack`: Stack of the thread.
    /// - `start_routine`: Pointer to the function to be executed by the thread.
    /// - `arg`: Argument to be passed to the thread function.
    ///
    /// # Behavior
    /// - The new thread shares the same address space as the calling thread.
    /// - The stack for the new thread is allocated automatically.
    pub fn thread_create(&self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        let name: String = todo!();
        let regs: Registers = todo!();

        let builder = ThreadBuilder::new(name);
        let tid = builder.get_tid();

        let task: Box<Thread> = todo!();

        builder.attach_task(task).spawn(move || regs.launch());
        Ok(tid as usize)
    }

    /// Wait for a thread to finish.
    ///
    /// This function blocks the calling thread until the specified thread
    /// terminates, and retrieves its exit code.
    ///
    /// Note that only a single call can receives the exit code of the dying
    /// thread. If multiple `thread_join` is called on the same thread,
    /// return values of others than the first one are InvalidArgument
    /// error.
    ///
    /// # Syscall API
    /// ```c
    /// int thread_join(int thread_id, int *retval);
    /// ```
    /// - `thread_id`: ID of the thread to join.
    /// - `retval`: Pointer to store the thread's exit code (optional).
    ///
    /// # Behavior
    /// - If the target thread has already exited, returns immediately with the
    ///   proper exit code.
    /// - If `retval` is non-null, the exit code of the target thread is stored.
    pub fn thread_join(&self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }

    /// Exit a process.
    ///
    /// This function terminates all the threads in the current process,
    /// including the current caller thread. The exit code is provided as
    /// the first argument (`arg1`) of the system call.
    ///
    /// # Syscall API
    /// ```c
    /// int exit_group(int status);
    /// ```
    /// - `status`: The thread's exit code.
    ///
    /// # Notes
    /// - This function does not return in normal execution, as it terminates
    ///   the process.
    /// - If an error occurs, it returns a `KernelError`
    pub fn exit_group(&self, abi: &SyscallAbi) -> Result<usize, KernelError> {
        todo!()
    }
}
