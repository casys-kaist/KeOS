//! Interrupt handler entries.

use super::Registers;
use crate::x86_64::{PrivilegeLevel, segmentation::SegmentSelector};
use core::{
    arch::{asm, global_asm},
    sync::atomic::Ordering,
};

global_asm!(include_str!("entry.s"));

unsafe extern "Rust" {
    fn kill_current_thread() -> !;
}

// Load interrupt descriptor table.
#[unsafe(no_mangle)]
#[allow(clippy::empty_loop)]
extern "C" fn handle_general_protection_fault(frame: &mut Registers, _c: SegmentSelector) {
    if _c.dpl() == PrivilegeLevel::Ring3 {
        unsafe {
            kill_current_thread();
        }
    } else {
        panic!("General Protection Fault! {:#?}", frame);
    }
}

#[unsafe(no_mangle)]
extern "C" fn handle_double_fault(
    frame: &mut Registers,
    _: crate::x86_64::interrupt::MustbeZero,
) -> ! {
    panic!("Double Fault!\n{:#?}", frame);
}

#[unsafe(no_mangle)]
extern "C" fn handle_nmi(frame: &mut Registers) {
    while crate::interrupt::NMI_EXPECTED_PANICKING.load(Ordering::SeqCst) {
        unsafe { asm!("cli; hlt") };
    }
    panic!("Unexpected NMI Interrupt!\n{:#?}", frame);
}

#[unsafe(no_mangle)]
extern "C" fn handle_invalid_opcode(frame: &mut Registers) {
    if frame.interrupt_stack_frame.cs.dpl() == PrivilegeLevel::Ring3 {
        unsafe {
            kill_current_thread();
        }
    } else {
        panic!("Invalid Opcode!\n{:#?}", frame);
    }
}

#[unsafe(no_mangle)]
extern "C" fn handle_simd_floating_point_exception(frame: &mut Registers) {
    if frame.interrupt_stack_frame.cs.dpl() == PrivilegeLevel::Ring3 {
        unsafe {
            kill_current_thread();
        }
    } else {
        panic!("SIMD Floating Point Exception!");
    }
}

#[unsafe(no_mangle)]
extern "C" fn handle_device_not_available(frame: &mut Registers) {
    if frame.interrupt_stack_frame.cs.dpl() == PrivilegeLevel::Ring3 {
        unsafe {
            kill_current_thread();
        }
    } else {
        panic!("Device Not Available");
    }
}

#[unsafe(no_mangle)]
#[allow(clippy::empty_loop)]
extern "C" fn do_handle_irq(frame: &mut Registers, vec: usize) {
    unsafe extern "Rust" {
        fn do_handle_interrupt(idx: usize, frame: &mut Registers);
    }

    crate::dev::x86_64::apic::eoi();

    if vec == 32 {
        unsafe {
            // Reprgram the deadline.
            crate::dev::x86_64::timer::set_timer();
        }
    }
    unsafe {
        do_handle_interrupt(vec - 32, frame);
    }
}
