//! Interrupt
#[cfg(doc)]
use crate::spinlock::SpinLockGuard;
use crate::{
    unwind,
    x86_64::{Rflags, interrupt::InterruptStackFrame, segmentation::Segment},
};
use core::{
    arch::{asm, naked_asm},
    sync::atomic::AtomicBool,
};

mod entry;

/// Enumeration representing the interrupt state.
#[derive(PartialEq, Eq, Debug)]
pub enum InterruptState {
    /// Interrupts are enabled.
    On,
    /// Interrupts are disabled.
    Off,
}

impl InterruptState {
    /// Reads the current interrupt state.
    ///
    /// # Returns
    /// - [`InterruptState::On`] if interrupts are enabled.
    /// - [`InterruptState::Off`] if interrupts are disabled.
    pub fn current() -> Self {
        if Rflags::read().contains(Rflags::IF) {
            Self::On
        } else {
            Self::Off
        }
    }
}

/// An RAII-based guard for managing interrupt disabling.
///
/// When an `InterruptGuard` is created, interrupts are disabled. When it is
/// dropped, the interrupt state is restored to what it was before the guard was
/// created.
///
/// **Important:**
/// - [`InterruptGuard`] instances **must be dropped in reverse order of their
///   creation** to prevent unintended interrupt state changes.
/// - Due to Rust's ownership and scoping rules, this invariant is naturally
///   upheld unless `drop()` is explicitly called prematurely or an
///   [`InterruptGuard`] is stored in a struct field.
///
/// This structure is created using [`InterruptGuard::new`].
pub struct InterruptGuard {
    state: InterruptState,
}

impl InterruptGuard {
    /// Creates a new `InterruptGuard`, disabling interrupts.
    ///
    /// # Behavior
    /// - Saves the current interrupt state.
    /// - Disables interrupts (`cli` instruction).
    ///
    /// # Returns
    /// A new instance of `InterruptGuard`, which will restore the original
    /// interrupt state when dropped.
    ///
    /// # Example
    /// ```rust
    /// let _guard = InterruptGuard::new(); // Disables interrupts
    /// // Critical section...
    /// // Interrupts are restored when `_guard` goes out of scope.
    /// ```
    pub fn new() -> Self {
        let state = InterruptState::current();
        unsafe {
            asm!("cli");
        }
        Self { state }
    }
}

impl Default for InterruptGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        if self.state == InterruptState::On {
            unsafe {
                asm!("sti");
            }
        }
    }
}

/// X86_64's general purpose registers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GeneralPurposeRegisters {
    /// R15 register.
    pub r15: usize,
    /// R14 register.
    pub r14: usize,
    /// R13 register.
    pub r13: usize,
    /// R12 register.
    pub r12: usize,
    /// R11 register.
    pub r11: usize,
    /// R10 register.
    pub r10: usize,
    /// R9 register.
    pub r9: usize,
    /// R8 register.
    pub r8: usize,
    /// RSI register.
    pub rsi: usize,
    /// RDI register.
    pub rdi: usize,
    /// RBP register.
    pub rbp: usize,
    /// RDX register.
    pub rdx: usize,
    /// RCX register.
    pub rcx: usize,
    /// RBX register.
    pub rbx: usize,
    /// RAX register.
    pub rax: usize,
}

/// x86_64 Trap frame.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Registers {
    pub gprs: GeneralPurposeRegisters,
    error_code: u64,
    #[doc(hidden)]
    pub interrupt_stack_frame: InterruptStackFrame,
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

impl Registers {
    /// Creates a new register frame for a user thread.
    ///
    /// This function initializes a [`Registers`] structure with default values
    /// for a new user-space thread.
    ///
    /// # Returns
    /// - A [`Registers`] instance with default values for user-space execution.
    ///
    /// # Example
    /// ```rust
    /// let mut regs = Registers::new_for_user();
    /// *regs.rip() = 0x400000; // Set entry point
    /// *regs.rsp() = 0x7FFFFFFFE000; // Set user stack pointer
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            gprs: GeneralPurposeRegisters::default(),
            error_code: 0,
            interrupt_stack_frame: InterruptStackFrame {
                rip: 0,                                /* Entry point of the user program should
                                                        * be set later. */
                cs: Segment::UserCode.into_selector(), // User-space code segment.
                __pad0: 0,
                __pad1: 0,
                rflags: Rflags::IF | Rflags::_1, // Enables interrupts.
                rsp: 0,                          /* User-space stack pointer should be set before
                                                  * execution. */
                ss: Segment::UserData.into_selector(), // User-space stack segment.
                __pad2: 0,
                __pad3: 0,
            },
        }
    }

    /// Returns a mutable reference to the instruction pointer (`RIP`).
    ///
    /// This function allows modifying the instruction pointer, which determines
    /// the next instruction the CPU will execute when the thread resumes.
    ///
    /// # Returns
    /// - A mutable reference to the `rip` field in the interrupt stack frame.
    ///
    /// # Example
    /// ```rust
    /// let mut regs = Registers::new_for_user();
    /// *regs.rip() = 0x400000; // Set the entry point
    /// ```
    pub fn rip(&mut self) -> &mut usize {
        &mut self.interrupt_stack_frame.rip
    }

    /// Returns a mutable reference to the stack pointer (`RSP`).
    ///
    /// This function allows modifying the stack pointer, which should point
    /// to the top of the stack before execution.
    ///
    /// # Returns
    /// - A mutable reference to the `rsp` field.
    ///
    /// # Example
    /// ```rust
    /// let mut regs = Registers::new_for_user();
    /// *regs.rsp() = 0x7FFFFFFFE000; // Set the user stack pointer
    /// ```
    pub fn rsp(&mut self) -> &mut usize {
        &mut self.interrupt_stack_frame.rsp
    }

    /// Launch the frame.
    ///
    /// Launches a thread by restoring its saved register state.
    ///
    /// This function returns the `never` type (`!`), meaning that once
    /// executed, there is no way to return to the current execution
    /// context.
    ///
    /// # Safety
    /// - The kernel must release all temporary resources such as locally
    ///   allocated `Box`, [`SpinLockGuard`], or [`InterruptGuard`] before
    ///   calling this function.
    ///
    /// # Behavior
    /// 1. Restores general-purpose registers from `self.gprs`.
    /// 2. Enables interrupts.
    /// 3. Transfers to saved execution state by executing `iretq`.
    ///
    /// # Example Usage
    /// ```rust
    /// let regs = Registers::new_for_user();
    /// regs.launch(); // This function does not return
    /// unreachable!() // Execution will never reach here
    #[unsafe(naked)]
    pub extern "C" fn launch(&self) -> ! {
        naked_asm!(
            "mov rax, [rdi + 0x70]",
            "mov rbx, [rdi + 0x68]",
            "mov rcx, [rdi + 0x60]",
            "mov rdx, [rdi + 0x58]",
            "mov rbp, [rdi + 0x50]",
            "mov rsi, [rdi + 0x40]",
            "mov r8, [rdi + 0x38]",
            "mov r9, [rdi + 0x30]",
            "mov r10, [rdi + 0x28]",
            "mov r11, [rdi + 0x20]",
            "mov r12, [rdi + 0x18]",
            "mov r13, [rdi + 0x10]",
            "mov r14, [rdi + 0x8]",
            "mov r15, [rdi]",
            "sti",
            "lea rsp, [rdi + 0x80]",
            "mov rdi, [rdi + 0x48]",
            "iretq"
        )
    }

    #[inline]
    #[doc(hidden)]
    pub fn to_stack_frame(&self) -> unwind::StackFrame {
        unwind::StackFrame {
            rax: self.gprs.rax,
            rbx: self.gprs.rbx,
            rcx: self.gprs.rcx,
            rdx: self.gprs.rdx,
            rsi: self.gprs.rsi,
            rdi: self.gprs.rdi,
            rbp: self.gprs.rbp,
            rsp: self.interrupt_stack_frame.rsp,
            r8: self.gprs.r8,
            r9: self.gprs.r9,
            r10: self.gprs.r10,
            r11: self.gprs.r11,
            r12: self.gprs.r12,
            r13: self.gprs.r13,
            r14: self.gprs.r14,
            r15: self.gprs.r15,
            rip: self.interrupt_stack_frame.rip,
        }
    }
}

impl core::fmt::Debug for Registers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(
            f,
            "RAX: {:016x} | RBX: {:016x}  | RCX: {:016x} | RDX: {:016x}\n\
             RSI: {:016x} | RDI: {:016x}  | RBP: {:016x} | RSP: {:016x}\n\
             R8 : {:016x} | R9 : {:016x}  | R10: {:016x} | R11: {:016x}\n\
             R12: {:016x} | R13: {:016x}  | R14: {:016x} | R15: {:016x}\n\
             RIP: {:016x} | Error Code: {:#x} | RFLAGS: {:016x} [{:?}]\n\
             CS:  {:?}   | SS: {:?}",
            self.gprs.rax,
            self.gprs.rbx,
            self.gprs.rcx,
            self.gprs.rdx,
            self.gprs.rsi,
            self.gprs.rdi,
            self.gprs.rbp,
            self.interrupt_stack_frame.rsp,
            self.gprs.r8,
            self.gprs.r9,
            self.gprs.r10,
            self.gprs.r11,
            self.gprs.r12,
            self.gprs.r13,
            self.gprs.r14,
            self.gprs.r15,
            self.interrupt_stack_frame.rip,
            self.error_code,
            self.interrupt_stack_frame.rflags.bits(),
            self.interrupt_stack_frame.rflags,
            self.interrupt_stack_frame.cs,
            self.interrupt_stack_frame.ss,
        )
    }
}

/// NMI Expection for Stopping CPUs on PANIC
pub static NMI_EXPECTED_PANICKING: AtomicBool = AtomicBool::new(false);
