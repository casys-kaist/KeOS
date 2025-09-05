//! APIC based timer.
use crate::addressing::Kva;
use crate::dev::DeviceError;
use crate::x86_64::{msr::Msr, pio::Pio};
use core::arch::x86_64::{__cpuid, _rdtsc, CpuidResult};

unsafe fn find_cpu_frequncy_lapic() -> Option<u64> {
    unsafe {
        Msr::<0x83E>::write(3); // APIC_REGISTER_TIMER_DIVIDE = 0b11 (Divide by 16)

        fn init_pit(val: u16) {
            // Calibrate through the PIT
            // Set the Gate high, disable speaker
            let chan2_gate = Pio::new(0x61);
            chan2_gate.write_u8((chan2_gate.read_u8() & !0x2) | 1);
            // Counter 2, mode 0 (one-shot), binary count
            Pio::new(0x43).write_u8(0xb0);
            Pio::new(0x42).write_u8(val as u8); // low byte
            Pio::new(0x42).write_u8((val >> 8) as u8); // high byte
        }
        init_pit(0xFFFF);

        Msr::<0x838>::write(0xFFFFFFFF); // APIC_REGISTER_TIMER_INIT_COUNT = -1 (i32)

        loop {
            fn read_pit() -> u16 {
                let low = Pio::new(0x42).read_u8() as u16;
                let high = Pio::new(0x42).read_u8() as u16;
                low | high << 8
            }

            if read_pit() < 0xffff - 0x4A9 {
                break;
            }
        }

        Msr::<0x832>::write(0x10000); // MASK
        Some(0xFFFFFFFF - Msr::<0x839>::read())
    }
}

/// Find cpu frequency
unsafe fn find_cpu_frequncy() -> Option<u64> {
    unsafe {
        unsafe fn kvm_find_cpuid_base() -> Option<u32> {
            unsafe {
                for base in (0x40000000..0x40010000).step_by(0x100) {
                    // If KVM,
                    let CpuidResult {
                        ebx: sig1,
                        ecx: sig2,
                        edx: sig3,
                        ..
                    } = __cpuid(base);

                    if u32::to_le_bytes(sig1) == *b"KVMK"
                        && u32::to_le_bytes(sig2) == *b"VMKV"
                        && u32::to_le_bytes(sig3) == *b"M\0\0\0"
                    {
                        return Some(base);
                    }
                }
                None
            }
        }

        // Using the kvm paravirtulized cpuid.
        if let Some(base) = kvm_find_cpuid_base() {
            #[repr(C, packed)]
            struct PvClockVcpuTimeInfo {
                version: u32,
                pad0: u32,
                tsc_timestamp: u64,
                system_time: u64,
                tsc_to_system_mul: u32,
                tsc_shift: i8,
                flags: u8,
                pad: [u8; 2],
            }
            #[repr(C, align(64))]
            struct Align<T>(T);
            static mut PV_INFO: Align<PvClockVcpuTimeInfo> = Align(PvClockVcpuTimeInfo {
                version: 0,
                pad0: 0,
                tsc_timestamp: 0,
                system_time: 0,
                tsc_to_system_mul: 0,
                tsc_shift: 0,
                flags: 0,
                pad: [0; 2],
            });
            // Has KVM_CLOCKSOURCE2 feature.
            if __cpuid(base | 0x40000001).eax & (1 << 3) != 0 {
                // MSR_KVM_SYSTEM_TIME_NEW
                Msr::<0x4b564d01>::write(
                    Kva::new(&raw mut PV_INFO.0 as *mut _ as usize)
                        .unwrap()
                        .into_pa()
                        .into_usize() as u64
                        | 1,
                );
            } else {
                // MSR_KVM_SYSTEM_TIME
                Msr::<0x12>::write(
                    Kva::new(&raw mut PV_INFO.0 as *mut _ as usize)
                        .unwrap()
                        .into_pa()
                        .into_usize() as u64
                        | 1,
                );
            }

            let tsc_khz = (1000000_u64 << 32) / (PV_INFO.0.tsc_to_system_mul as u64);

            return if PV_INFO.0.tsc_shift < 0 {
                Some(tsc_khz << (-PV_INFO.0.tsc_shift as u64))
            } else {
                Some(tsc_khz >> (PV_INFO.0.tsc_shift as u64))
            };
        }
        // "Borrowed" from linux's quick_pit_calibrate() in /arch/x86/kernel/tsc.c
        {
            const MAX_QUICK_PIT_ITERATIONS: u64 = 50 * 1193182 / 1000 / 256;

            fn verify_msb(val: u8) -> bool {
                let _ = Pio::new(0x42).read_u8();
                Pio::new(0x42).read_u8() == val
            }
            unsafe fn expect_msb(val: u8) -> Option<(u64, u64)> {
                unsafe {
                    let (mut count, mut prev_tsc, mut tsc) = (0, 0, 0);

                    while count < 50000 {
                        if !verify_msb(val) {
                            break;
                        }
                        prev_tsc = tsc;
                        tsc = _rdtsc();
                        count += 1;
                    }
                    let delta = _rdtsc() - prev_tsc;
                    if count > 5 { Some((tsc, delta)) } else { None }
                }
            }

            // Calibrate through the PIT
            // Set the Gate high, disable speaker
            let chan2_gate = Pio::new(0x61);
            chan2_gate.write_u8((chan2_gate.read_u8() & !0x2) | 1);
            // Counter 2, mode 0 (one-shot), binary count
            Pio::new(0x43).write_u8(0xb0);
            // Start at 0xffff
            Pio::new(0x42).write_u8(0xff);
            Pio::new(0x42).write_u8(0xff);

            // The PIT starts counting at the next edge, so we
            // need to delay for a microsecond. The easiest way
            // to do that is to just read back the 16-bit counter
            // once from the PIT.
            verify_msb(0);

            if let Some((tsc, d1)) = expect_msb(0xff) {
                for i in 1..=MAX_QUICK_PIT_ITERATIONS as u8 {
                    if let Some((mut delta, d2)) = expect_msb(0xff - i) {
                        delta -= tsc;

                        if i == 1 && d1 + d2 >= ((delta * MAX_QUICK_PIT_ITERATIONS) >> 11) {
                            break;
                        }
                        if d1 + d2 >= (delta >> 11) {
                            continue;
                        }
                        if !verify_msb(0xfe - i) {
                            break;
                        }
                        return Some(delta * 1193182 / (i as u64 * 256 * 1000));
                    } else {
                        break;
                    }
                }
            }
        }

        None
    }
}

static mut CPU_FREQ: u64 = 0;

/// Internal Timer Mode
#[derive(Clone, Copy)]
enum TimerMode {
    OneShot,
    TSCDeadline,
}

/// Which timer the system is using?
static mut GLOBAL_TIMER_MODE: Option<TimerMode> = None;

/// Initialize the timer system.
pub unsafe fn init(core_id: usize) -> Result<(), DeviceError> {
    unsafe {
        if core::arch::x86_64::__cpuid(1).ecx & (1 << 24) != 0 {
            if core_id == 0 {
                CPU_FREQ =
                    find_cpu_frequncy().ok_or(DeviceError("Failed to find cpu frequency."))?;
                GLOBAL_TIMER_MODE = Some(TimerMode::TSCDeadline);
            }
            // Timer
            // Irq #32.
            Msr::<0x832>::write((0b10 << 17) | 32);
            set_timer();
            Ok(())
        } else {
            if core_id == 0 {
                warning!("tsc timer is not supported; falling back to LAPIC timer");
                CPU_FREQ = find_cpu_frequncy_lapic()
                    .ok_or(DeviceError("Failed to find cpu frequency."))?;
                GLOBAL_TIMER_MODE = Some(TimerMode::OneShot);
            }

            // Timer
            // Irq #32.
            Msr::<0x832>::write(32);
            set_timer();
            Ok(())
        }
    }
}

/// Program the deadline timer.
pub unsafe fn set_timer() {
    unsafe {
        // TscDeadline
        // 1ms resolution.

        let mode = GLOBAL_TIMER_MODE.unwrap();
        match mode {
            TimerMode::OneShot => {
                Msr::<0x838>::write(CPU_FREQ);
            }
            TimerMode::TSCDeadline => {
                let next = _rdtsc() + CPU_FREQ;
                Msr::<0x6e0>::write(next);
            }
        }

        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    }
}
