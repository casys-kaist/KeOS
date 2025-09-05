//! KEOS panic handler.
use crate::thread::STACK_SIZE;
use abyss::{
    dev::x86_64::apic::{IPIDest, Mode, send_ipi},
    interrupt::{InterruptGuard, NMI_EXPECTED_PANICKING},
    unwind::{DwarfReader, Peeker, StackFrame, UnwindBacktrace},
    x86_64::{intrinsics::cpuid, kernel_gs, pio::Pio},
};
use addr2line::{Context, Frame};
use alloc::{borrow::Cow, sync::Arc};
use core::mem::ManuallyDrop;
use core::sync::atomic::Ordering;

#[derive(Clone)]
struct EhFrameReader;

impl EhFrameReader {
    fn start() -> usize {
        unsafe extern "C" {
            static __eh_frame_hdr_start: u8;
        }
        unsafe { &__eh_frame_hdr_start as *const _ as usize }
    }

    fn end() -> usize {
        unsafe extern "C" {
            static __eh_frame_end: u8;
        }
        unsafe { &__eh_frame_end as *const _ as usize }
    }
}

impl Peeker for EhFrameReader {
    fn read<T>(&self, ofs: usize) -> Option<T>
    where
        T: Copy,
    {
        let (start, end) = (Self::start(), Self::end());
        if ofs >= start && ofs + core::mem::size_of::<T>() < end {
            unsafe { (ofs as *const T).as_ref().cloned() }
        } else {
            None
        }
    }
}

struct BackTracePrinter<'a>(
    Frame<'a, gimli::EndianArcSlice<gimli::LittleEndian>>,
    Option<u64>,
);

impl core::fmt::Display for BackTracePrinter<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(pc) = self.1 {
            if let Some(Ok(name)) = self.0.function.as_ref().map(|n| n.demangle()) {
                writeln!(formatter, "{name} [0x{pc:016x}]")?;
            } else {
                writeln!(formatter, "0x{pc:016x}")?;
            }
        }
        if let Some(file) = self.0.location.as_ref().and_then(|n| n.file) {
            write!(formatter, "                         at {file}:")?;
        } else {
            write!(formatter, "                         at ?:")?;
        }
        if let Some(line) = self.0.location.as_ref().and_then(|n| n.line) {
            write!(formatter, "{line}:")?;
        } else {
            write!(formatter, "?:")?;
        }
        if let Some(col) = self.0.location.as_ref().and_then(|n| n.column) {
            write!(formatter, "{col}")
        } else {
            write!(formatter, "?")
        }
    }
}

fn do_backtrace(state: &mut (isize, bool), frame: &StackFrame) {
    let (depth, has_non_kernel_addr) = state;
    let pc = frame.pc() as u64;
    if pc >> 48 != 0xffff {
        *has_non_kernel_addr = true;
    }
    *depth += 1;
    if *depth == -1 {
        return; /* skip `abyss::unwind::x86_64::StackFrame::current` */
    }
    if let Some(ctxt) = unsafe { DEBUG_CONTEXT.as_ref() }
        && let Ok(mut frames) = ctxt.find_frames(pc)
        && let Ok(Some(frame)) = frames.next()
    {
        println!(
            "  {:2}: {}",
            depth,
            BackTracePrinter(
                frame,
                Some(
                    pc + 1, /* For a normal call frame we need to back up so we point within the
                            call itself; this is important because a) the call might be the
                            very last instruction of the function and the edge of the FDE,
                            and b) so that run_cfi_program() runs locations up to the call
                            but not more. */
                )
            ),
        );
        while let Ok(Some(frame)) = frames.next() {
            println!("{}", BackTracePrinter(frame, None));
        }
        return;
    }
    println!("  {:2}: 0x{:016x}  - ?", depth, pc);
}

static mut DEBUG_CONTEXT: Option<Context<gimli::EndianArcSlice<gimli::LittleEndian>>> = None;

#[allow(dead_code)]
#[allow(unreachable_code)]
#[allow(clippy::empty_loop)]
#[inline(never)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Disabling preempt before we go on.
    let _cli = ManuallyDrop::new(InterruptGuard::new());

    match crate::PANIC_DEPTH.fetch_add(1, Ordering::SeqCst) {
        0 => {}
        1 => {
            unsafe {
                abyss::kprint::force_unlock_serial();
            }
            println!("*** PANIC recursed: Forcing the shutdown ***");
            println!("{}", info);
            unsafe {
                abyss::x86_64::power_control::power_off();
            }
        }
        _ => unsafe {
            abyss::x86_64::power_control::power_off();
        },
    }
    // Asserting expected NMI for every CPU except the faulting CPU.
    //
    // # Safety
    // NEVER use print!() or println!() before force serial unlock
    NMI_EXPECTED_PANICKING.store(true, Ordering::SeqCst);
    for (cpuid, is_online) in abyss::boot::ONLINE_CPU
        .iter()
        .enumerate()
        .filter(|(id, o)| o.load(Ordering::SeqCst) && *id != cpuid())
    {
        unsafe {
            send_ipi(IPIDest::Cpu(cpuid), Mode::Nmi);
        }
        is_online.store(false, Ordering::SeqCst);
    }
    // Forcefully remove kprint serial lock to prevent deadlock.
    unsafe {
        abyss::kprint::force_unlock_serial();
    }
    fn panic_internal_poweroff(has_non_kernel_addr: bool) -> ! {
        if has_non_kernel_addr {
            println!("\nWARNING: Non-kernel address detected in the backtrace.\n");
            println!(
                "It indicates that some of addresses above are either user program's or even completely wrong."
            );
            println!(
                "For details, please refer to `Debugging a User Process` chapter of the KeOS documentation."
            );
        }
        println!("\nShutting down the system in few moment...");
        for _ in 0..50 {
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

            loop {
                fn read_pit() -> u16 {
                    let low = Pio::new(0x42).read_u8() as u16;
                    let high = Pio::new(0x42).read_u8() as u16;
                    low | high << 8
                }
                if read_pit() == 0 {
                    break;
                }
            }
        }
        unsafe {
            abyss::x86_64::power_control::power_off();
        }
    }

    let frame = StackFrame::current();

    if unsafe { ((frame.sp() & !(STACK_SIZE - 1)) as *mut crate::thread::ThreadStack).as_mut() }
        .map(|stack| stack.magic == crate::thread::THREAD_MAGIC)
        .unwrap_or(false)
    {
        crate::thread::with_current(|th| {
            println!(
                "\n\nKeOS thread '{}' [core #{}, tid #{}] {}\n",
                th.name,
                abyss::x86_64::intrinsics::cpuid(),
                th.tid,
                info
            );
        });
    } else {
        println!(
            "\n\nKeOS thread '<unknown>' [core #{}, tid #?] {}\n",
            abyss::x86_64::intrinsics::cpuid(),
            info
        );
    }

    println!("Stack Backtrace: ");
    let mut state = (-2, false);
    let sp_hi = frame.sp() & !(STACK_SIZE - 1);
    if let Err(e) = unsafe {
        UnwindBacktrace::new(
            frame,
            sp_hi..sp_hi + STACK_SIZE,
            DwarfReader::from_peeker(EhFrameReader::start(), EhFrameReader),
        )
        .run(&mut state, |state, this, _| {
            do_backtrace(state, &this.frame)
        })
    } {
        println!("** Backtrace Failed: {:?}", e);
    }
    if let Some(trap_frame) = unsafe { kernel_gs::current().interrupt_frame.as_ref() } {
        println!("Triggered by event at: ");
        let frame = trap_frame.to_stack_frame();
        let sp_hi = frame.sp() & !(STACK_SIZE - 1);
        if let Err(e) = unsafe {
            UnwindBacktrace::new(
                frame,
                sp_hi..sp_hi + STACK_SIZE,
                DwarfReader::from_peeker(EhFrameReader::start(), EhFrameReader),
            )
            .run(&mut state, |depth, this, _| {
                do_backtrace(depth, &this.frame)
            })
        } {
            println!("** Backtrace Failed: {:?}", e);
        }
    }
    panic_internal_poweroff(state.1)
}

/// Load debugging symbols from kernel image
#[allow(clippy::result_unit_err)]
pub(crate) fn load_debug_infos() -> bool {
    use object::{Object, ObjectSection};
    let Some(kernel_disk) = abyss::dev::get_bdev(0) else {
        return false;
    };
    let image_size = kernel_disk.block_cnt() * kernel_disk.block_size();
    let mut kernel_image = alloc::vec![0u8; image_size].into_boxed_slice();
    if kernel_disk
        .read_bios(&mut Some((0, kernel_image.as_mut())).into_iter())
        .is_err()
    {
        return false;
    }
    let Ok(kernel) = object::File::parse(kernel_image.as_ref()) else {
        return false;
    };
    let Ok(dwarf): Result<_, ()> = gimli::Dwarf::load(|id| {
        let data = kernel
            .section_by_name(id.name())
            .and_then(|section| section.uncompressed_data().ok())
            .unwrap_or(Cow::Borrowed(&[]));
        let data: Arc<[u8]> = Arc::from(data.as_ref());
        Ok(gimli::EndianArcSlice::new(data, gimli::LittleEndian))
    }) else {
        return false;
    };
    unsafe {
        DEBUG_CONTEXT = Context::from_dwarf(dwarf).ok();
        DEBUG_CONTEXT.is_some()
    }
}
