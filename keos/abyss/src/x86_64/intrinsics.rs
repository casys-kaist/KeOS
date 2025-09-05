//! Intrinsics of x86_64 not included in [`core::arch::x86_64`].
//!
//! [`core::arch::x86_64`]: https://doc.rust-lang.org/beta/core/arch/x86_64/index.html
use core::arch::asm;

/// Get cpuid of this core.
pub fn cpuid() -> usize {
    unsafe { (core::arch::x86_64::__cpuid(1).ebx >> 24) as usize }
}

/// read current cr3.
pub fn read_cr3() -> usize {
    unsafe {
        let r: u64;
        asm!("mov {}, cr3", out(reg) r);
        r as usize
    }
}
