//! System Power RAW Operation.

use core::arch::asm;

use crate::x86_64::pio::Pio;

/// `machine_restart()` is emergency restart function since it lacks
/// any synchorization technique.
///
/// This function exploits PS/2 I/O to touch the reset pin at the hardware, but
/// if it fails, it tries to intentionally generate the triple fault by loading
/// invalid IDT (Interrupt Descriptor Table) and invoke interrupt.
///
/// It's intended to be run at the end of safe restart or emergency situation.
///
/// This function should be run with interrupt disabled.
pub unsafe fn restart() -> ! {
    // Make sure that External Interrupt is disabled before we do the actual
    // operation, regardless of actual CPU state.
    unsafe {
        asm!("cli");
    }

    let mut good = 0x02;
    let port = Pio::new(0x64);
    let mut ps2_try_cnt = 0;
    while (good & 0x02) == 0x02 {
        good = port.read_u8();
        ps2_try_cnt += 1;
        if ps2_try_cnt > 20 {
            break;
        }
    }
    if (good & 0x02) != 0x02 {
        port.write_u8(0xFE);
    }

    for _ in 0..0x1000 {
        core::hint::spin_loop();
    }

    // 8042 Restart failed! Try to triple fault
    unsafe {
        asm!("lidt [0]");
        asm!("int3");
        core::hint::unreachable_unchecked();
    }
}

/// `machine_power_off()` is emergency shutdown function since it lacks
/// any synchorization technique.
///
/// This function will try to put system in ACPI power down (S5), but if it
/// fails, it will try to use emulator ports to power down the system.
/// If every power down techniques are unavailable, it fall backs to halt.
///
/// It's intended to be run at the end of safe restart or emergency situation.
///
/// This function should be run with interrupt disabled.
pub unsafe fn power_off() -> ! {
    // Make sure that External Interrupt is disabled before we do the actual
    // operation, regardless of actual CPU state.
    unsafe {
        asm!("cli");
    }

    // Trying QEMU specific power off method
    let emulators = [
        (Pio::new(0xB004), 0x2000),
        (Pio::new(0x604), 0x2000),
        (Pio::new(0x3004), 0x3400),
        (Pio::new(0x600), 0x34),
    ];
    for emulator in emulators {
        emulator.0.write_u16(emulator.1);
    }

    loop {
        // Power off failed!
        unsafe {
            asm!("hlt");
        }
    }
}
