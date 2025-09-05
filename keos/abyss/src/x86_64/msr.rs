//! Model-specific register (MSR).

use core::arch::asm;

/// Model specific register.
pub struct Msr<const ADDR: usize>;

impl<const ADDR: usize> Msr<ADDR> {
    /// Read the current value.
    #[inline(always)]
    pub fn read() -> u64 {
        let hi: u32;
        let lo: u32;
        unsafe {
            asm!("rdmsr", out("edx") hi, out("eax") lo, in("ecx") ADDR, options(nomem, nostack));
            ((hi as u64) << 32) | (lo as u64)
        }
    }

    /// Write to the msr.
    #[inline(always)]
    pub unsafe fn write(v: u64) {
        unsafe {
            asm!(
                "wrmsr",
                in("edx") (v >> 32) as u32,
                in("eax") v as u32,
                in("ecx") ADDR,
                options(nomem, nostack)
            );
        }
    }
}
