//! Thread scheduler

use super::{ParkHandle, STACK_SIZE, THREAD_MAGIC, Thread, ThreadStack, ThreadState};
use abyss::spinlock::SpinLock;
use alloc::boxed::Box;
use core::{arch::asm, sync::atomic::AtomicBool};

/// A trait for a thread scheduler.
///
/// The [`Scheduler`] trait defines the common functionality expected from a
/// thread scheduler. It provides an interface for managing threads, determining
/// which thread to run next, and handling periodic timer interrupts. A thread
/// scheduler is responsible for controlling the execution of threads in a
/// system. The scheduler determines when each thread is allowed to run, how to
/// handle context switching, and ensures fair allocation of CPU time among all
/// threads.
///
/// This trait can be implemented by different types of schedulers, such as
/// Round Robin, Priority-based, or Multi-level Queue schedulers. Each
/// implementation may have a unique strategy for selecting the next
/// thread to run and handling thread management.
pub trait Scheduler {
    /// Peek a next thread to run.
    ///
    /// This method checks the queue and returns the next thread to run. If no
    /// threads are available, it returns `None`.
    ///
    /// # Returns
    ///
    /// Returns an `Option<Box<Thread>>` containing the next thread to run or
    /// `None` if no threads are available to execute.
    fn next_to_run(&self) -> Option<Box<Thread>>;

    /// Push a thread `th` into scheduling queue.
    ///
    /// This method adds the specified thread to the queue of threads waiting to
    /// be scheduled.
    ///
    /// # Arguments
    ///
    /// * `th` - A boxed [`Thread`] object that represents the thread to be
    ///   added to the scheduler's queue.
    fn push_to_queue(&self, th: Box<Thread>);

    /// Called on every timer interrupt (1ms).
    ///
    /// This method is triggered by the timer interrupt (e.g., every 1ms) and
    /// allows the scheduler to manage time slices, perform context
    /// switching, or adjust thread priorities as needed.
    fn timer_tick(&self);
}

pub(crate) static mut SCHEDULER: Option<&'static dyn Scheduler> = None;

/// A First-in-first-out scheduler.
struct Fifo {
    runqueue: SpinLock<alloc::collections::VecDeque<Box<Thread>>>,
}

unsafe impl core::marker::Sync for Fifo {}

impl Scheduler for Fifo {
    fn next_to_run(&self) -> Option<Box<Thread>> {
        let mut guard = self.runqueue.lock();
        let val = guard.pop_front();
        guard.unlock();
        val
    }
    fn push_to_queue(&self, th: Box<Thread>) {
        let mut guard = self.runqueue.lock();
        guard.push_back(th);
        guard.unlock();
    }
    fn timer_tick(&self) {}
}

static FIFO: Fifo = Fifo {
    runqueue: SpinLock::new(alloc::collections::VecDeque::new()),
};

/// Set the scheduler of the kernel.
pub(crate) unsafe fn set_scheduler(t: impl Scheduler + 'static) {
    unsafe {
        SCHEDULER = (Box::into_raw(Box::new(t)) as *const dyn Scheduler).as_ref();
    }
}

/// Get the reference of the kernel scheduler.
pub fn scheduler() -> &'static (dyn Scheduler + 'static) {
    if let Some(sched) = unsafe { SCHEDULER.as_mut() } {
        *sched
    } else {
        &FIFO
    }
}

impl dyn Scheduler {
    /// Reschedule the current thread.
    pub fn reschedule(&self) {
        assert!(
            !abyss::interrupt::InterruptGuard::is_guarded(),
            "Try to reschedule a thread while holding a lock."
        );

        unsafe { abyss::interrupt::InterruptState::disable() };
        match self.next_to_run() {
            Some(th) => {
                th.run();
            }
            _ => unsafe {
                IDLE[abyss::x86_64::intrinsics::cpuid()]
                    .as_mut()
                    .unwrap()
                    .do_run();
            },
        }
        unsafe { abyss::interrupt::InterruptState::enable() };
    }

    /// Park a thread 'th' and return ParkHandle.
    pub(crate) unsafe fn park_thread(&self, th: &mut Thread) -> Result<ParkHandle, ()> {
        let mut state = th.state.lock();
        if matches!(*state, ThreadState::Parked) {
            return Err(());
        }
        *state = ThreadState::Parked;
        state.unlock();
        unsafe {
            Ok(ParkHandle {
                th: Box::from_raw(th),
            })
        }
    }
}

pub(crate) static BOOT_DONE: AtomicBool = AtomicBool::new(false);
const INIT: Option<Box<Thread>> = None;
static mut IDLE: [Option<Box<Thread>>; abyss::MAX_CPU] = [INIT; abyss::MAX_CPU];

/// Transmute this thread into the idle.
pub(crate) fn idle(core_id: usize) -> ! {
    let mut sp: usize;
    unsafe {
        asm!("mov {}, rsp", out(reg) sp);
    }

    let mut tcb = Thread::new("idle");
    let mut state = tcb.state.lock();
    *state = ThreadState::Idle;
    state.unlock();

    tcb.stack = unsafe { Box::from_raw((sp & !(STACK_SIZE - 1)) as *mut ThreadStack) };
    tcb.stack.magic = THREAD_MAGIC;
    tcb.stack.thread = tcb.as_mut() as *mut _;
    unsafe {
        IDLE[core_id] = Some(tcb);
    }

    while !BOOT_DONE.load(core::sync::atomic::Ordering::SeqCst) {
        core::hint::spin_loop();
    }
    let scheduler = crate::thread::scheduler::scheduler();
    loop {
        if let Some(th) = scheduler.next_to_run() {
            th.run();
        }
        #[cfg(not(feature = "gkeos"))]
        unsafe {
            asm!("sti", "hlt", "cli")
        }
    }
}
