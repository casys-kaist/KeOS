//! x86_64 specific

pub mod interrupt;
pub mod intrinsics;
pub mod kernel_gs;
pub mod msr;
pub mod pio;
pub mod power_control;
pub mod segmentation;
pub mod table;
pub mod tss;

use core::arch::asm;

/// Privilege Levels.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PrivilegeLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

bitflags::bitflags! {
    /// rflags.
    #[repr(transparent)]
    pub struct Rflags: u64 {
        /// Carry Flag
        const CF = 1 << 0;
        /// Must be 1.
        const _1 = 1 << 1;
        /// Parity Flag
        const PF = 1 << 2;
        /// Adjust Flag
        const AF = 1 << 4;
        /// Zero Flag
        const ZF = 1 << 6;
        /// Sign Flag
        const SF = 1 << 7;
        /// Trap Flag
        const TF = 1 << 8;

        /// Interrupt enable.
        ///
        /// Controls the response of the processor to maskable hardware
        /// interrupt  requests (see also: Section 6.3.2, "Maskable
        /// Hardware Interrupts"). The flag is set to respond to maskable
        /// hardware interrupts; cleared to inhibit maskable hardware
        /// interrupts. The IF flag does not affect the generation of exceptions
        /// or nonmaskable interrupts (NMI interrupts). The CPL, IOPL, and the
        /// state of the VME  flag in control register CR4 determine
        /// whether the IF flag can be modified by the CLI, STI, POPF, POPFD,
        /// and IRET.
        const IF = 1 << 9;

        /// Direction Flag
        const DF = 1 << 10;
        /// Overflow Flag
        const OF = 1 << 11;

        /// I/O privilege level field - bit 0
        ///
        /// Indicates the I/O privilege level (IOPL) of the currently
        /// running program or task. The CPL of the currently running program or
        /// task must be less than or equal to the IOPL to access the
        /// I/O address space. The POPF and IRET instructions can modify this
        /// field only when operating at a CPL of 0.
        ///
        /// The IOPL is also one of the mechanisms that controls the
        /// modification of the IF flag and the handling of interrupts
        /// in virtual-8086 mode when virtual mode extensions are in effect
        /// (when CR4.VME = 1).
        ///
        /// See also: Chapter 18, "Input/Output," in
        /// the Intel® 64 and IA-32 Architectures Software Developer’s Manual,
        /// Volume 1.
        const IOPL0 = 1 << 12;
        /// I/O privilege level field - bit 1
        ///
        /// Indicates the I/O privilege level (IOPL) of the currently
        /// running program or task. The CPL of the currently running program or
        /// task must be less than or equal to the IOPL to access the
        /// I/O address space. The POPF and IRET instructions can modify this
        /// field only when operating at a CPL of 0.
        /// The IOPL is also one of the mechanisms that controls the
        /// modification of the IF flag and the handling of interrupts
        /// in virtual-8086 mode when virtual mode extensions are in effect
        /// (when CR4.VME = 1).
        ///
        /// See also: Chapter 18, "Input/Output," in the Intel® 64 and IA-32
        /// Architectures Software Developer’s Manual, Volume 1.
        const IOPL1 = 1 << 13;
        /// Nested task
        ///
        /// Controls the chaining of interrupted and called tasks. The processor
        /// sets this flag on calls to a task initiated with a CALL
        /// instruction, an interrupt, or an exception. It examines and modifies
        /// this flag on returns from a task initiated with the IRET
        /// instruction. The flag can be explicitly set or cleared
        /// with the POPF/POPFD instructions; however, changing to the state of
        /// this flag can generate unexpected exceptions in application
        /// programs.
        ///
        /// See also: Section 7.4, "Task Linking."
        const NT = 1 << 14;
        /// Resume
        ///
        /// Controls the processor’s response to instruction-breakpoint
        /// conditions. When set, this flag temporarily disables debug
        /// exceptions (#DB) from being generated for instruction breakpoints
        /// (although other exception conditions can cause an exception to be
        /// generated). When clear, instruction breakpoints will
        /// generate debug exceptions.
        ///
        /// The primary function of the RF flag is to allow the restarting of an
        /// instruction following a debug exception that was caused by
        /// an instruction breakpoint condition. Here, debug software must set
        /// this flag in the EFLAGS image on the stack just prior to
        /// returning to the interrupted program with IRETD (to prevent the
        /// instruction breakpoint from causing another debug exception). The
        /// processor then automatically clears this flag after the
        /// instruction returned to has been successfully executed, enabling
        /// instruction breakpoint faults again.
        ///
        /// See also: Section 17.3.1.1, "Instruction-Breakpoint Exception
        /// Condition."
        const RF = 1 << 16;
        /// Virtual-8086 mode
        ///
        /// Set to enable virtual-8086 mode; clear to return to protected mode.
        ///
        /// See also: Section 20.2.1, "Enabling Virtual-8086 Mode."
        const VM = 1 << 17;
        /// Alignment check or access control
        ///
        /// If the AM bit is set in the CR0 register, alignment
        /// checking of user-mode data accesses is enabled if and only if this flag is 1. An alignment-check exception
        /// is generated when reference is made to an unaligned operand, such as a word at an odd byte address or a
        /// doubleword at an address which is not an integral multiple of four. Alignment-check exceptions are generated only in user mode (privilege level 3). Memory references that default to privilege level 0, such as
        /// segment descriptor loads, do not generate this exception even when caused by instructions executed in
        /// user-mode.
        ///
        /// The alignment-check exception can be used to check alignment of data. This is useful when exchanging
        /// data with processors which require all data to be aligned. The alignment-check exception can also be used
        /// by interpreters to flag some pointers as special by misaligning the pointer. This eliminates overhead of
        /// checking each pointer and only handles the special pointer when used.
        ///
        /// If the SMAP bit is set in the CR4 register, explicit supervisor-mode data accesses to user-mode pages are
        /// allowed if and only if this bit is 1. See Section 4.6, "Access Rights."
        const AC = 1 << 18;
        /// Virtual Interrupt
        ///
        /// Contains a virtual image of the IF flag. This flag is used in conjunction with
        /// the VIP flag. The processor only recognizes the VIF flag when either the VME flag or the PVI flag in control
        /// register CR4 is set and the IOPL is less than 3. (The VME flag enables the virtual-8086 mode extensions;
        /// the PVI flag enables the protected-mode virtual interrupts.)
        ///
        /// See also: Section 20.3.3.5, "Method 6: Software Interrupt Handling," and Section 20.4, "Protected-Mode
        /// Virtual Interrupts."
        const VIF = 1 << 19;
        /// Virtual interrupt pending
        ///
        /// Set by software to indicate that an interrupt is pending; cleared to
        /// indicate that no interrupt is pending. This flag is used in conjunction with the VIF flag. The processor reads
        /// this flag but never modifies it. The processor only recognizes the VIP flag when either the VME flag or the
        /// PVI flag in control register CR4 is set and the IOPL is less than 3. The VME flag enables the virtual-8086
        /// mode extensions; the PVI flag enables the protected-mode virtual interrupts.
        ///
        /// See Section 20.3.3.5, "Method 6: Software Interrupt Handling," and Section 20.4, "Protected-Mode Virtual
        /// Interrupts."
        const VIP = 1 << 20;
        /// Identification.
        ///
        /// The ability of a program or procedure to set or clear this flag
        /// indicates support for the CPUID instruction.
        const ID = 1 << 21;
    }
}

impl Rflags {
    /// Read the current value.
    #[inline(always)]
    pub fn read() -> Self {
        let ret: u64;
        unsafe {
            asm!(
                "pushf",
                "pop {0}",
                lateout(reg) ret,
            );
            Self::from_bits_truncate(ret)
        }
    }
}

bitflags::bitflags! {
    /// Cr0 Register.
    #[repr(transparent)]
    pub struct Cr0: u64 {
        /// Protected mode enable.
        const PE = 1 << 0;
        /// Monitor co-processor.
        const MP = 1 << 1;
        /// Emulation.
        const EM = 1 << 2;
        /// Task switched.
        const TS = 1 << 3;
        /// Extension type.
        const ET = 1 << 4;
        /// Numeric error.
        const NE = 1 << 5;
        /// Write protect.
        const WP = 1 << 16;
        /// Alignment mask.
        const AM = 1 << 18;
        /// Not-write through.
        const NW = 1 << 29;
        /// Cache disable.
        const CD = 1 << 30;
        /// Paging.
        const PG = 1 << 31;
    }
}

impl Cr0 {
    /// Read the current value.
    #[inline(always)]
    pub fn current() -> Self {
        let ret: u64;
        unsafe {
            asm!("mov {}, cr0", lateout(reg) ret, options(nomem, nostack));
            Self::from_bits_unchecked(ret)
        }
    }

    /// Write the current value.
    ///
    /// # Safety
    /// Write to system register is unsafe.
    #[inline(always)]
    pub unsafe fn apply(self) {
        unsafe {
            asm!("mov cr0, {}", in(reg) self.bits(), options(nomem, nostack));
        }
    }
}

/// ControlRegister2 Register.
///
/// Contains Page-Fault Linear Address.
#[derive(Debug)]
pub struct Cr2(u64);

impl Cr2 {
    /// Read the current value.
    #[inline(always)]
    pub fn current() -> Self {
        let ret: u64;
        unsafe {
            asm!("mov {}, cr2", lateout(reg) ret, options(nomem, nostack));
            Self(ret)
        }
    }

    /// Cast into usize.
    #[inline(always)]
    pub const fn into_usize(self) -> usize {
        self.0 as usize
    }
}

/// ControlRegister3 Register. Contains Page-Table root.
#[derive(Debug, PartialEq)]
pub struct Cr3(pub u64);

impl Cr3 {
    /// Read the current value.
    #[inline(always)]
    pub fn current() -> Self {
        let ret: u64;
        unsafe {
            asm!("mov {}, cr3", lateout(reg) ret, options(nomem, nostack));
            Self(ret)
        }
    }

    /// Cast into usize.
    #[inline(always)]
    pub const fn into_usize(self) -> usize {
        self.0 as usize
    }

    /// Write the current value.
    ///
    /// # Safety
    /// Write to system register is unsafe.
    #[inline(always)]
    pub unsafe fn apply(self) {
        unsafe {
            asm!("mov cr3, {}", in(reg) self.0, options(nomem, nostack));
        }
    }
}

bitflags::bitflags! {
    /// Cr4 Register.
    #[repr(transparent)]
    pub struct Cr4: u64 {
        /// Virtual 8086 mode extensions.
        const VME = 1 << 0;
        /// Protected mode virtual interrupts.
        const PVI = 1 << 1;
        /// Time stamp disable.
        const TSD = 1 << 2;
        /// Debugging extensions.
        const DE = 1 << 3;
        /// Page size extension.
        const PSE = 1 << 4;
        /// Physical address extension.
        const PAE = 1 << 5;
        /// Machine check exception.
        const MCE = 1 << 6;
        /// Page global enable.
        const PGE = 1 << 7;
        /// Performance monitoring counter enable.
        const PCE = 1 << 8;
        /// Os support for fxsave and fxrstor instructions.
        const OSFXSR = 1 << 9;
        /// Os support for unmasked simd floating point exceptions.
        const OSXMMEXCPT = 1 << 10;
        /// User mode instruction prevention (#GP on SGDT, SIDT, SLDT, SMSW, and STR instructions when CPL > 0).
        const UMIP = 1 << 11;
        /// Virtual machine extensions enable.
        const VMXE = 1 << 13;
        /// Safer mode extensions enable.
        const SMXE = 1 << 14;
        /// Pcid enable.
        const PCIDE = 1 << 17;
        /// Xsave and processor extended states enable.
        const OSXSAVE = 1 << 18;
        /// Supervisor mode executions protection enable.
        const SMEP = 1 << 20;
        /// Supervisor mode access protection enable.
        const SMAP = 1 << 21;
        /// Protection keys for user-mode pages enable.
        const PKE = 1 << 22;
        /// Control-flow-enforcement enable.
        const CET = 1 << 23;
        /// Protection keys for supervisor-mode pages enable.
        const PKS = 1 << 24;
    }
}

impl Cr4 {
    /// Read the current value.
    #[inline(always)]
    pub fn current() -> Self {
        let ret: u64;
        unsafe {
            asm!("mov {}, cr4", lateout(reg) ret, options(nomem, nostack));
            Self::from_bits_unchecked(ret)
        }
    }

    /// Read the current value.
    ///
    /// # Safety
    /// Write to system register is unsafe.
    #[inline(always)]
    pub unsafe fn apply(self) {
        unsafe {
            asm!("mov cr4, {}", in(reg) self.bits(), options(nomem, nostack));
        }
    }
}
