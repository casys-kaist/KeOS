//! Thread abstration, an abstraction of a cpu core.
//!
//! ## The threading model
//!
//! An executing kernel consists of a collection of threads,
//! each with their own stack and local state. Threads can be named, and
//! provide some built-in support for low-level synchronization.
pub mod scheduler;

use crate::{KernelError, spinlock::SpinLock, task::Task};
use abyss::{
    addressing::{Kva, Pa},
    dev::x86_64::apic::{IPIDest, Mode},
    interrupt::InterruptGuard,
    x86_64::intrinsics::cpuid,
};
use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc};
use core::{
    arch::{asm, naked_asm},
    panic::Location,
    sync::atomic::{AtomicI32, AtomicU64, Ordering},
};

/// Size of each thread's stack.
pub const STACK_SIZE: usize = 0x100000;
/// Thread magic to detect stack overflow.
pub const THREAD_MAGIC: usize = 0xdeadbeefcafebabe;

/// The Thread stack.
///
/// DO NOT MODIFY THIS STRUCT.
#[repr(C, align(0x100000))]
#[doc(hidden)]
pub(crate) struct ThreadStack {
    pub(crate) thread: *mut Thread,
    pub(crate) magic: usize,
    /// Padding to fill up to [`STACK_SIZE`]
    pub(crate) _pad:
        [u8; STACK_SIZE - core::mem::size_of::<*mut Thread>() - core::mem::size_of::<usize>()],
    /// Marker of address of usable stack.
    pub(crate) _usable_marker: [u8; 0],
    /// Pinned.
    _pin: core::marker::PhantomPinned,
}

/// A possible state of the thread.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum ThreadState {
    /// Thread is runnable.
    Runnable,
    /// Thread is running.
    Running,
    /// Thread is exited with exitcode.
    Exited(i32),
    /// Thread is idle.
    Idle,
    /// Thread is parked.
    Parked,
}

pub(crate) struct TtyState {
    input: &'static [u8],
    idx: usize,
    output: String,
}

impl crate::teletype::Teletype for TtyState {
    fn write(&mut self, data: &[u8]) -> Result<usize, KernelError> {
        if let Ok(s) = String::from_utf8(data.to_vec()) {
            self.output.push_str(&s);
            Ok(data.len())
        } else {
            Err(KernelError::InvalidArgument)
        }
    }

    fn read(&mut self, data: &mut [u8]) -> Result<usize, KernelError> {
        let read_bytes = self.input.len().wrapping_sub(self.idx).min(data.len());
        data[..read_bytes].copy_from_slice(&self.input[self.idx..self.idx + read_bytes]);
        self.idx += read_bytes;
        Ok(read_bytes)
    }
}

fn load_pt(pa: Pa) {
    unsafe { abyss::x86_64::Cr3(pa.into_usize() as u64).apply() }
}

static EXIT_CODE_TABLE: SpinLock<BTreeMap<u64, Arc<AtomicU64>>> = SpinLock::new(BTreeMap::new());
static THREAD_STATE_TABLE: SpinLock<BTreeMap<u64, Arc<SpinLock<ThreadState>>>> =
    SpinLock::new(BTreeMap::new());

#[unsafe(no_mangle)]
#[doc(hidden)]
pub fn kill_current_thread() -> ! {
    unsafe {
        __do_exit(-1);
    }
}

#[unsafe(no_mangle)]
#[doc(hidden)]
pub unsafe fn __do_exit(exit_code: i32) -> ! {
    let _ = abyss::interrupt::InterruptGuard::new();
    with_current(|th| {
        let mut et = EXIT_CODE_TABLE.lock();
        et.remove(&th.tid);
        et.unlock();

        let mut tst = THREAD_STATE_TABLE.lock();
        tst.remove(&th.tid);
        tst.unlock();

        th.exit_status
            .store(0x8000_0000_0000_0000 | (exit_code as u64), Ordering::SeqCst);
        let mut state = th.state.lock();
        *state = ThreadState::Exited(exit_code);
        state.unlock();
        scheduler::scheduler().reschedule();
    });
    unreachable!()
}

/// Check signal for current process and perform an exit when signaled.
pub(crate) fn __check_for_signal() {
    let _ = __with_current(|th| {
        let exit_status = th.exit_status.load(Ordering::SeqCst);
        if (exit_status & 0x4000_0000_0000_0000) == 0x4000_0000_0000_0000 {
            unsafe {
                __do_exit(exit_status as i32);
            }
        }
    });
}

/// Kill the thread by specified TID (Thread ID).
pub fn kill_by_tid(tid: u64, exit_code: i32) -> Result<(), KernelError> {
    let et = EXIT_CODE_TABLE.lock();
    let Some(exit_status) = et.get(&tid) else {
        et.unlock();
        return Err(KernelError::InvalidArgument);
    };
    let exit_status = exit_status.clone();
    et.unlock();

    debug_assert_eq!(
        exit_status.load(Ordering::SeqCst) & 0x4000_0000_FFFF_FFFF,
        0
    );

    exit_status.store(0x4000_0000_0000_0000 | exit_code as u64, Ordering::SeqCst);

    unsafe {
        abyss::dev::x86_64::apic::send_ipi(IPIDest::AllExcludingSelf, Mode::Fixed(0x7f));
    }

    Ok(())
}

/// Get specified thread's [`ThreadState`] by TID (Thread ID).
pub fn get_state_by_tid(tid: u64) -> Result<ThreadState, KernelError> {
    let tst = THREAD_STATE_TABLE.lock();

    let Some(state) = tst.get(&tid) else {
        tst.unlock();
        return Err(KernelError::InvalidArgument);
    };

    let ts_lock = state.lock();
    let result = *ts_lock;

    ts_lock.unlock();
    tst.unlock();

    Ok(result)
}

#[repr(C)]
/// An thread abstraction.
pub struct Thread {
    /// A stack pointer on context switch.
    ///
    /// ## WARNING
    /// DO NOT CHANGE THE OFFSET THIS FIELDS.
    /// This offset used in context switch with hard-coded value.
    /// You must add your own members **BELOWS** this sp field.
    pub(crate) sp: usize,
    /// Thread Stack
    pub(crate) stack: Box<ThreadStack>,
    /// Thread id
    pub tid: u64,
    /// Thread name
    pub name: String,
    /// State of the thread.
    pub state: Arc<SpinLock<ThreadState>>,
    pub(crate) running_cpu: Arc<AtomicI32>,
    /// Mixture of exit state (63th and 62th bit) and exit code (lower 32 bits).
    pub exit_status: Arc<AtomicU64>,
    /// Interrupt Frame if thread was handling interrupt.
    pub interrupt_frame: SpinLock<*const abyss::interrupt::Registers>,
    #[doc(hidden)]
    pub task: Option<Box<dyn Task>>,
    // Grading utils.
    pub(crate) tty_hook: SpinLock<Option<Arc<SpinLock<TtyState>>>>,
    pub(crate) allocations: SpinLock<Option<BTreeMap<Kva, &'static Location<'static>>>>,
}

impl Thread {
    #[doc(hidden)]
    pub fn new<I>(name: I) -> Box<Self>
    where
        alloc::string::String: core::convert::From<I>,
    {
        static TID: AtomicU64 = AtomicU64::new(0);
        let tid = TID.fetch_add(1, Ordering::SeqCst);
        let mut stack: Box<ThreadStack> = unsafe { Box::new_uninit().assume_init() };
        stack.magic = THREAD_MAGIC;

        let exit_status = Arc::new(AtomicU64::new(0));
        let mut et = EXIT_CODE_TABLE.lock();
        et.insert(tid, exit_status.clone());
        et.unlock();

        let state = Arc::new(SpinLock::new(ThreadState::Runnable));
        let mut tst = THREAD_STATE_TABLE.lock();
        tst.insert(tid, state.clone());
        tst.unlock();

        Box::new(Self {
            sp: 0,
            stack,
            tid,
            name: String::from(name),
            state,
            exit_status,
            interrupt_frame: SpinLock::new(core::ptr::null()),
            running_cpu: Arc::new(AtomicI32::new(-1)),
            task: None,
            tty_hook: SpinLock::new(
                __with_current(|th| {
                    let guard = th.tty_hook.lock();
                    let val = guard.as_ref().map(|n| n.clone());
                    guard.unlock();
                    val
                })
                .unwrap_or(None),
            ),
            allocations: SpinLock::new(None),
        })
    }

    #[doc(hidden)]
    pub fn track_alloc(&self) {
        let mut guard = self.allocations.lock();
        *guard = Some(BTreeMap::new());
        guard.unlock();
    }

    #[doc(hidden)]
    pub fn validate_alloc(&self) {
        let guard = self.allocations.lock();
        if let Some(allocs) = guard.as_ref()
            && !allocs.is_empty()
        {
            struct DebugAlloc<'a>(&'a BTreeMap<Kva, &'static Location<'static>>);
            impl core::fmt::Debug for DebugAlloc<'_> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    writeln!(f, "List of unallocated pages:")?;
                    for (idx, (va, loc)) in self.0.iter().take(10).enumerate() {
                        writeln!(f, "  {idx}: {va:?}, allocated at {loc}")?;
                    }
                    if self.0.len() > 10 {
                        write!(f, "... and {:?} more allocations.", self.0.len() - 10)?;
                    }
                    Ok(())
                }
            }
            panic!(
                "Grader: Validating `Page` allocation state failed: Detecting {} non-freed pages.\n{:?}",
                allocs.len(),
                DebugAlloc(allocs)
            );
        }
        guard.unlock();
    }

    pub(crate) unsafe fn do_run(&mut self) {
        unsafe {
            let _p = abyss::interrupt::InterruptGuard::new();
            if with_current(|current| current as *const _ as usize != self as *const _ as usize) {
                let next_sp = self.sp;
                let current_sp = with_current(|th| {
                    while self.running_cpu.load(Ordering::SeqCst) != -1 {
                        core::hint::spin_loop()
                    }
                    &mut th.sp as *mut usize
                });
                assert_eq!(
                    abyss::interrupt::InterruptState::current(),
                    abyss::interrupt::InterruptState::Off
                );
                context_switch_trampoline(current_sp, next_sp)
            }
        }
    }

    pub(crate) fn run(self: Box<Self>) {
        unsafe { Box::into_raw(self).as_mut().unwrap().do_run() }
    }

    /// Pin current thread not to be scheduled by blocking interrupt.
    ///
    /// When [`ThreadPinGuard`] is dropped, the current thread is unpinned.
    /// When you hold multiple [`ThreadPinGuard`], you **MUST** drops
    /// [`ThreadPinGuard`] as a reverse order of creation.
    pub fn pin() -> ThreadPinGuard {
        ThreadPinGuard::new()
    }

    #[doc(hidden)]
    pub fn hook_stdin(&self, b: &'static [u8]) {
        let mut guard = self.tty_hook.lock();
        if guard.is_some() {
            panic!("Fail to hook stdin: already hook.");
        } else {
            *guard = Some(Arc::new(SpinLock::new(TtyState {
                input: b,
                idx: 0,
                output: String::new(),
            })));
            guard.unlock();
        }
    }

    #[doc(hidden)]
    pub fn finish_hook(&self) -> Option<String> {
        let mut guard = self.tty_hook.lock();
        let val = guard.take().map(|n| {
            let guard = n.lock();
            let val = alloc::borrow::ToOwned::to_owned(&(*guard.output));
            guard.unlock();
            val
        });
        guard.unlock();
        val
    }
}

/// A RAII implementation of the thread pinning.
pub type ThreadPinGuard = InterruptGuard;

/// A handle to join thread.
pub struct JoinHandle
where
    Self: 'static,
{
    /// Thread id of this handle.
    pub tid: u64,
    exit_status: Arc<AtomicU64>,
    running_cpu: Arc<AtomicI32>,
}

impl JoinHandle {
    /// Make a join handle for Thread `th`.
    pub fn new_for(th: &Thread) -> Self {
        Self {
            tid: th.tid,
            exit_status: th.exit_status.clone(),
            running_cpu: th.running_cpu.clone(),
        }
    }

    /// Join this handle and returns exit code.
    pub fn join(self) -> i32 {
        loop {
            let v = self.exit_status.load(Ordering::SeqCst);
            if v >= 0x8000_0000_0000_0000 {
                return v as i32;
            }
            crate::scheduler().reschedule();
        }
    }

    /// Get scheudled cpu id of the underlying thread.
    ///
    /// If the thread is not runnig, returns None.
    pub fn try_get_running_cpu(&self) -> Option<usize> {
        match self.running_cpu.load(Ordering::SeqCst) {
            v if v < 0 => None,
            v => Some(v as usize),
        }
    }
}

unsafe impl Send for JoinHandle {}
unsafe impl Sync for JoinHandle {}

/// A handle that represent the parked thread.
pub struct ParkHandle {
    pub(crate) th: Box<Thread>,
}

impl ParkHandle {
    pub(crate) fn new_for(th: Box<Thread>) -> Self {
        Self { th }
    }

    /// Consume the handle and unpark the underlying thread.
    pub fn unpark(self) {
        // Wait until context switch is finished.
        while self.th.running_cpu.load(Ordering::SeqCst) != -1 {
            core::hint::spin_loop()
        }
        let mut state = self.th.state.lock();
        *state = ThreadState::Runnable;
        state.unlock();

        scheduler::scheduler().push_to_queue(self.th);
    }
}

unsafe impl Send for ParkHandle {}
unsafe impl Sync for ParkHandle {}

// Context switch related codes.

/// The context-switch magic.
#[unsafe(naked)]
unsafe extern "C" fn context_switch_trampoline(_current_sp: *mut usize, _next_sp: usize) {
    // XXX: we don't need to rflags because when threads entered this function the
    // rflags state is always same. RDI: Current Stack pointer storage.
    // RSI: Next Stack pointer.
    naked_asm!(
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // Switch.
        "mov r8, rsp",
        "mov [rdi], r8",
        "mov rsp, rsi",

        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",

        // XXX: Tail-call optimization, pass prev thread to rdi
        "jmp {}",
        sym finish_context_switch
    );
}

unsafe extern "C" fn finish_context_switch(prev: &'static mut Thread) {
    unsafe {
        assert_eq!(
            abyss::interrupt::InterruptState::current(),
            abyss::interrupt::InterruptState::Off
        );

        let prev_state = {
            let lock = prev.state.lock();
            let result = *lock;
            lock.unlock();
            result
        };

        let mut prev_interrupt_frame = prev.interrupt_frame.lock();
        *prev_interrupt_frame = abyss::x86_64::kernel_gs::current().interrupt_frame;
        prev_interrupt_frame.unlock();

        let _dropped = match prev_state {
            ThreadState::Exited(_e) => Some(Box::from_raw(prev)),
            ThreadState::Idle => None,
            ThreadState::Running => {
                let mut prev_state = prev.state.lock();
                *prev_state = ThreadState::Runnable;
                prev_state.unlock();

                let th = Box::from_raw(prev);
                scheduler::scheduler().push_to_queue(th);
                None
            }
            ThreadState::Parked => None,
            ThreadState::Runnable => unreachable!("{:?} {:?}", prev as *const _, prev.name),
        };
        with_current(|th| {
            let mut state = th.state.lock();
            if *state != ThreadState::Idle {
                *state = ThreadState::Running
            }
            state.unlock();

            __check_for_signal();

            let interrupt_frame = th.interrupt_frame.lock();
            abyss::x86_64::kernel_gs::current().interrupt_frame = *interrupt_frame;
            interrupt_frame.unlock();
            
            abyss::x86_64::segmentation::SegmentTable::update_tss(
                th.stack.as_mut() as *mut _ as usize + STACK_SIZE,
            );
            th.running_cpu.store(cpuid() as i32, Ordering::SeqCst);

            if let Some(task) = th.task.as_mut() {
                task.with_page_table_pa(&(load_pt as fn(Pa)));
            } else {
                unsafe extern "C" {
                    static mut boot_pml4e: u64;
                }
                load_pt(Pa::new(boot_pml4e as usize).unwrap());
            }
        });
        prev.running_cpu.store(-1, Ordering::SeqCst);
        drop(_dropped);
    }
}

#[inline]
pub(crate) fn __with_current<R>(f: impl FnOnce(&mut Thread) -> R) -> Result<R, usize> {
    unsafe {
        let mut sp: usize;
        asm!("mov {}, rsp", out(reg) sp);
        if let Some(stack) = ((sp & !(STACK_SIZE - 1)) as *mut ThreadStack).as_mut()
            && stack.magic == THREAD_MAGIC
        {
            return Ok(f(stack.thread.as_mut().unwrap()));
        }
        Err(sp)
    }
}

/// The opaque structure indicating the running thread on the current cpu.
pub struct Current {
    _p: (),
}

impl Current {
    /// Run a function `f` with [`ParkHandle`] for current thread, and then park
    /// the current thread.
    pub fn park_with(f: impl FnOnce(ParkHandle)) {
        with_current(|th| {
            f(unsafe { scheduler::scheduler().park_thread(th).unwrap() });
        });
        assert!(
            abyss::interrupt::InterruptState::current() == abyss::interrupt::InterruptState::On,
            "Try to park a thread while holding a lock."
        );
        let _ = abyss::interrupt::InterruptGuard::new();
        scheduler::scheduler().reschedule();
    }

    /// Exit the current thread with `exit_code`.
    pub fn exit(exit_code: i32) -> ! {
        assert!(
            abyss::interrupt::InterruptState::current() == abyss::interrupt::InterruptState::On,
            "Try to exit a thread while holding a lock."
        );
        unsafe {
            __do_exit(exit_code);
        }
    }

    /// Get the current thread's id.
    pub fn get_tid() -> u64 {
        with_current(|th| th.tid)
    }
}

/// Run a function `f` with current thread as an argument.
#[inline]
pub fn with_current<R>(f: impl FnOnce(&mut Thread) -> R) -> R {
    __with_current(f).unwrap_or_else(|sp| {
        panic!(
            "Stack overflow detected! You might allocate big local variables. Stack: {:x}",
            sp
        )
    })
}

/// A struct to mimic a stack state on context switch.
#[repr(C)]
struct ContextSwitchFrame<F: FnOnce() + Send> {
    _r15: usize,
    _r14: usize,
    _r13: usize,
    _r12: usize,
    _bx: usize,
    _bp: usize,
    ret_addr: usize,
    thread_fn: *mut F,
    end_of_stack: usize,
}

/// A struct to build a new thread.
pub struct ThreadBuilder {
    th: Box<Thread>,
}

impl ThreadBuilder {
    /// Create a new thread builder for thread `name`.
    pub fn new<I>(name: I) -> Self
    where
        alloc::string::String: core::convert::From<I>,
    {
        Self {
            th: Thread::new(name),
        }
    }

    /// Attach a task to the thread.
    pub fn attach_task(mut self, task: Box<dyn Task>) -> Self {
        self.th.task = Some(task);
        self
    }

    /// Spawn the thread as a parked state.
    pub fn spawn_as_parked<F: FnOnce() + Send + 'static>(self, thread_fn: F) -> ParkHandle {
        let th = self.into_thread(thread_fn);
        ParkHandle::new_for(th)
    }

    /// Spawn the thread.
    pub fn spawn<F: FnOnce() + Send + 'static>(self, thread_fn: F) -> JoinHandle {
        let th = self.into_thread(thread_fn);
        let handle = JoinHandle::new_for(&th);
        scheduler::scheduler().push_to_queue(th);
        handle
    }

    /// Get the thread id of this thread.
    pub fn get_tid(&self) -> u64 {
        self.th.tid
    }

    fn into_thread<F: FnOnce() + Send + 'static>(self, thread_fn: F) -> Box<Thread> {
        /// The very beginning of the thread
        #[unsafe(naked)]
        unsafe extern "C" fn start<F: FnOnce() + Send>() -> ! {
            naked_asm!(
                "pop rdi",
                "sti",
                "jmp {}",
                sym thread_start::<F>
            );
        }

        fn thread_start<F: FnOnce() + Send>(thread_fn: *mut F) {
            unsafe {
                core::intrinsics::catch_unwind(
                    |o| {
                        let o = *Box::from_raw(o as *mut F);
                        o();
                        Current::exit(0);
                    },
                    thread_fn as *mut u8,
                    |_, _| {},
                );
            }
        }

        let Self { mut th } = self;
        let stack = th.stack.as_mut();
        let frame = unsafe {
            ((&mut stack._usable_marker as *mut _ as usize
                - core::mem::size_of::<ContextSwitchFrame<F>>())
                as *mut ContextSwitchFrame<F>)
                .as_mut()
                .unwrap()
        };
        frame.end_of_stack = 0;
        frame.thread_fn = Box::into_raw(Box::new(thread_fn));
        frame.ret_addr = start::<F> as usize;
        th.sp = frame as *mut _ as usize;
        th.stack.thread = th.as_mut() as *mut _;
        th
    }
}
