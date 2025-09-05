//! Port mapped io interface

use core::arch::asm;

/// A Port-mapped io.
#[derive(Clone, Copy)]
pub struct Pio(u16);

impl Pio {
    /// Create a new port mapped io interface.
    #[inline(always)]
    pub const fn new(port: u16) -> Self {
        Pio(port)
    }

    /// Read u8 from port.
    #[inline(always)]
    pub fn read_u8(self) -> u8 {
        unsafe {
            let ret: u8;
            asm!("in al, dx", lateout("al") ret, in("dx") self.0, options(nomem, nostack));
            ret
        }
    }

    /// Read u16 from port.
    #[inline(always)]
    pub fn read_u16(self) -> u16 {
        unsafe {
            let ret: u16;
            asm!("in ax, dx", lateout("eax") ret, in("dx") self.0, options(nomem, nostack));
            ret
        }
    }

    /// Read u32 from port.
    #[inline(always)]
    pub fn read_u32(self) -> u32 {
        unsafe {
            let ret: u32;
            asm!("in eax, dx", lateout("eax") ret, in("dx") self.0, options(nomem, nostack));
            ret
        }
    }

    /// Write u8 to port.
    #[inline(always)]
    pub fn write_u8(self, data: u8) {
        unsafe {
            asm!("out dx, al", in("al") data, in("dx") self.0, options(nomem, nostack));
        }
    }

    /// Write u16 to port.
    #[inline(always)]
    pub fn write_u16(self, data: u16) {
        unsafe {
            asm!("out dx, ax", in("eax") data, in("dx") self.0, options(nomem, nostack));
        }
    }

    /// Write u32 to port.
    #[inline(always)]
    pub fn write_u32(self, data: u32) {
        unsafe {
            asm!("out dx, eax", in("eax") data, in("dx") self.0, options(nomem, nostack));
        }
    }

    /// Read multiple u32s from port.
    #[inline(always)]
    pub fn read_u32_multiple(self, addr: u32, cnt: u32) {
        unsafe {
            asm!(
                "cld",
                "repnz ins DWORD PTR es:[rdi], dx",
                in("dx") self.0,
                in("edi") addr,
                in("ecx") cnt,
                lateout("edi") _,
                lateout("ecx") _,
                options(nostack)
            );
        }
    }
}
