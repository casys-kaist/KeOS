#![no_std]
extern crate grading_derive;

pub use grading_derive::*;
/// Execute a syscall instruction with given arguments.
#[macro_export]
macro_rules! syscall {
    ($nr:expr_2021) => {
        unsafe {
            let mut result: isize;
            core::arch::asm!(
                "syscall",
                in("ax") $nr,
                lateout("ax") result,
                lateout("rcx") _,
                lateout("r11") _,
                options(nostack)
            );
            result
        }
    };
    ($nr:expr_2021, $arg1:expr_2021) => {
        unsafe {
            let mut result: isize;
            core::arch::asm!(
                "syscall",
                in("ax") $nr,
                in("di") $arg1,
                lateout("ax") result,
                lateout("rcx") _,
                lateout("r11") _,
                options(nostack)
            );
            result
        }
    };
    ($nr:expr_2021, $arg1:expr_2021, $arg2:expr_2021) => {
        unsafe {
            let mut result: isize;
            core::arch::asm!(
                "syscall",
                in("ax") $nr,
                in("di") $arg1,
                in("si") $arg2,
                lateout("ax") result,
                lateout("rcx") _,
                lateout("r11") _,
                options(nostack)
            );
            result
        }
    };
    ($nr:expr_2021, $arg1:expr_2021, $arg2:expr_2021, $arg3:expr_2021) => {
        unsafe {
            let mut result: isize;
            core::arch::asm!(
                "syscall",
                in("ax") $nr,
                in("di") $arg1,
                in("si") $arg2,
                in("dx") $arg3,
                lateout("ax") result,
                lateout("rcx") _,
                lateout("r11") _,
                options(nostack)
            );
            result
        }
    };
    ($nr:expr_2021, $arg1:expr_2021, $arg2:expr_2021, $arg3:expr_2021, $arg4:expr_2021) => {
        unsafe {
            let mut result: isize;
            core::arch::asm!(
                "syscall",
                in("ax") $nr,
                in("di") $arg1,
                in("si") $arg2,
                in("dx") $arg3,
                in("r10") $arg4,
                lateout("ax") result,
                lateout("rcx") _,
                lateout("r11") _,
                options(nostack)
            );
            result
        }
    };
    ($nr:expr_2021, $arg1:expr_2021, $arg2:expr_2021, $arg3:expr_2021, $arg4:expr_2021, $arg5:expr_2021) => {
        unsafe {
            let mut result: isize;
            core::arch::asm!(
                "syscall",
                in("ax") $nr,
                in("di") $arg1,
                in("si") $arg2,
                in("dx") $arg3,
                in("r10") $arg4,
                in("r8") $arg5,
                lateout("ax") result,
                lateout("rcx") _,
                lateout("r11") _,
                options(nostack)
            );
            result
        }
    };
    ($nr:expr_2021, $arg1:expr_2021, $arg2:expr_2021, $arg3:expr_2021, $arg4:expr_2021, $arg5:expr_2021, $arg6:expr_2021) => {
        unsafe {
            let mut result: isize;
            core::arch::asm!(
                "syscall",
                in("ax") $nr,
                in("di") $arg1,
                in("si") $arg2,
                in("dx") $arg3,
                in("r10") $arg4,
                in("r8") $arg5,
                in("r9") $arg6,
                lateout("ax") result,
                lateout("rcx") _,
                lateout("r11") _,
                options(nostack)
            );
            result
        }
    };
}
