//! Booting sequence

mod multiboot;

use crate::{
    MAX_CPU,
    addressing::{Kva, Pa},
    dev::x86_64::apic::{IPIDest, Mode},
    syscall::syscall_entry_register,
    x86_64::{
        Cr0,
        interrupt::{ExceptionType, IDT, InterruptStackFrame, PFErrorCode},
        kernel_gs::KernelGS,
        msr::Msr,
        pio::Pio,
        segmentation::{Segment, SegmentSelector},
    },
};
use core::{
    arch::{global_asm, naked_asm},
    sync::atomic::Ordering,
};
use core::{ops::Range, sync::atomic::AtomicBool};
use multiboot::MultiBootInfo2;

/// A physically contigous memory region.
#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct Region {
    pub addr: Range<Pa>,
    pub usable: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct Regions {
    pub regions: [Region; 64],
    pub size: usize,
}

impl Regions {
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Region> + '_ {
        self.regions.iter().take(self.size)
    }
}

global_asm!(include_str!("bootstrap.s"));
global_asm!(include_str!("ap.s"));

unsafe extern "Rust" {
    fn rust_main(core_id: usize, mbinfo: Regions, cmd: Option<&'static [u8]>);
    fn rust_ap_main(core_id: usize);
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
unsafe extern "C" fn start() {
    naked_asm!(
        "lea rax, [rip + IDLE_STACK]", // rax = IDLE_STACK
        "mov rcx, rdi", // rcx = core_id
        "inc rcx",      // rcx = core_id + 1
        "shl rcx, {}",  // rcx = (core_id + 1) << STACK_SIZE_SHIFT
        "add rax, rcx", // rax = IDLE_STACK[core_id + 1]. stack is grow downward.
        "mov rsp, rax", // rsp = rax
        "jmp {}",       // jump to rust main
        const 0x100000i32.trailing_zeros(),
        sym bootstrap,
    );
}

/// Bootup the mps.
pub unsafe fn bootup_mps() {
    unsafe extern "C" {
        static ap_trampoline: u8;
        static ap_trampoline_end: u8;
        static mut boot_pml4e: u64;
    }
    const MP_ENTRY: u32 = 0x8000;

    unsafe {
        boot_pml4e = crate::x86_64::intrinsics::read_cr3() as u64;

        // Init bsp.
        Pio::new(0x70).write_u8(0xF); // cmos port.
        Pio::new(0x71).write_u8(0xA); // cmos command.
        core::ptr::copy_nonoverlapping(
            &ap_trampoline as *const u8,
            Pa::new(MP_ENTRY as usize).unwrap().into_kva().into_usize() as *mut u8,
            (&ap_trampoline_end as *const _ as usize) - (&ap_trampoline as *const _ as usize),
        );

        // Setup warm reset vector.
        let (hi, lo) = (
            Pa::new(0x469).unwrap().into_kva().into_usize() as *mut u16,
            Pa::new(0x467).unwrap().into_kva().into_usize() as *mut u16,
        );

        // XXX: core::ptr::write_volatile PANICs when the address is not aligned on
        // debug mode. Therefore, use core::intrinsics::volatile_store.
        core::intrinsics::volatile_store(lo, (MP_ENTRY >> 4) as u16);
        core::intrinsics::volatile_store(hi, (MP_ENTRY as u16) & 0xf);

        // Bootup mps.
        for mpid in 1..MAX_CPU {
            crate::dev::x86_64::apic::send_ipi(IPIDest::Cpu(mpid), Mode::Init);
            crate::dev::x86_64::apic::send_ipi(
                IPIDest::Cpu(mpid),
                Mode::StartUp((MP_ENTRY >> 12) as u8),
            );
            crate::dev::x86_64::apic::send_ipi(
                IPIDest::Cpu(mpid),
                Mode::StartUp((MP_ENTRY >> 12) as u8),
            );
        }
    }
    info!("Bootup Application Processors.");
    // spinning until all aps are booted
    for cpu_state in ONLINE_CPU.iter().skip(1) {
        while !cpu_state.load(Ordering::SeqCst) {
            core::hint::spin_loop();
        }
    }
}

unsafe extern "C" fn bootstrap(core_id: usize, mbinfo: &MultiBootInfo2) {
    unsafe {
        if core_id == 0 {
            // Cleanup the bss.
            crate::dev::x86_64::serial::init();
            unsafe extern "C" {
                static __edata_start: u64;
                static __edata_end: u64;
            }

            let start = Kva::new(&__edata_start as *const _ as usize)
                .unwrap()
                .into_usize();
            let end = Kva::new(&__edata_end as *const _ as usize)
                .unwrap()
                .into_usize();
            core::slice::from_raw_parts_mut(start as *mut u8, end - start).fill(0);

            initialize_idt();
        }

        per_cpu_init(core_id);

        if core_id == 0 {
            let cmd = mbinfo.get_cmdline();
            if let Some(cmd) = cmd
                .and_then(|n| core::ffi::CStr::from_bytes_until_nul(n).ok())
                .and_then(|n| n.to_str().ok())
            {
                crate::QUITE.store(cmd.split(" ").any(|n| n == "--quite"), Ordering::SeqCst);
            }
            rust_main(
                core_id,
                Regions::from(mbinfo.get_memory_map().expect("Failed to read memory info")),
                cmd,
            );
        } else {
            rust_ap_main(core_id);
        }
    }
}

pub static ONLINE_CPU: [AtomicBool; MAX_CPU] = [const { AtomicBool::new(false) }; MAX_CPU];

unsafe fn per_cpu_init(core_id: usize) {
    unsafe {
        use crate::x86_64::segmentation::SEGMENT_TABLE;
        IDT.load();
        SEGMENT_TABLE.load();
        SEGMENT_TABLE.init_tss();

        crate::dev::x86_64::apic::init(core_id).expect("Failed to initialize apic");
        crate::dev::x86_64::timer::init(core_id).expect("Failed to initialize timer");

        // WP: Protect Readonly page from kernel's write.
        // TS: #nm when use the fpu.
        ((Cr0::current() & !(Cr0::CD | Cr0::NW)) | Cr0::WP | Cr0::TS | Cr0::NE).apply();
        // Init EFER
        // bit0: System Call Extensions.
        // bit11: No-Execute Enable.
        Msr::<0xc0000080>::write(Msr::<0xc0000080>::read() | (1 << 11) | (1 << 0));

        KernelGS::new().apply();
        syscall_entry_register();

        ONLINE_CPU[core_id].store(true, Ordering::SeqCst);
    }
}

fn initialize_idt() {
    unsafe extern "x86-interrupt" {
        fn do_isr_32(_: &mut InterruptStackFrame);
        fn do_isr_33(_: &mut InterruptStackFrame);
        fn do_isr_34(_: &mut InterruptStackFrame);
        fn do_isr_35(_: &mut InterruptStackFrame);
        fn do_isr_36(_: &mut InterruptStackFrame);
        fn do_isr_37(_: &mut InterruptStackFrame);
        fn do_isr_38(_: &mut InterruptStackFrame);
        fn do_isr_39(_: &mut InterruptStackFrame);
        fn do_isr_40(_: &mut InterruptStackFrame);
        fn do_isr_41(_: &mut InterruptStackFrame);
        fn do_isr_42(_: &mut InterruptStackFrame);
        fn do_isr_43(_: &mut InterruptStackFrame);
        fn do_isr_44(_: &mut InterruptStackFrame);
        fn do_isr_45(_: &mut InterruptStackFrame);
        fn do_isr_46(_: &mut InterruptStackFrame);
        fn do_isr_47(_: &mut InterruptStackFrame);
        fn do_isr_48(_: &mut InterruptStackFrame);
        fn do_isr_49(_: &mut InterruptStackFrame);
        fn do_isr_50(_: &mut InterruptStackFrame);
        fn do_isr_51(_: &mut InterruptStackFrame);
        fn do_isr_52(_: &mut InterruptStackFrame);
        fn do_isr_53(_: &mut InterruptStackFrame);
        fn do_isr_54(_: &mut InterruptStackFrame);
        fn do_isr_55(_: &mut InterruptStackFrame);
        fn do_isr_56(_: &mut InterruptStackFrame);
        fn do_isr_57(_: &mut InterruptStackFrame);
        fn do_isr_58(_: &mut InterruptStackFrame);
        fn do_isr_59(_: &mut InterruptStackFrame);
        fn do_isr_60(_: &mut InterruptStackFrame);
        fn do_isr_61(_: &mut InterruptStackFrame);
        fn do_isr_62(_: &mut InterruptStackFrame);
        fn do_isr_63(_: &mut InterruptStackFrame);
        fn do_isr_64(_: &mut InterruptStackFrame);
        fn do_isr_65(_: &mut InterruptStackFrame);
        fn do_isr_66(_: &mut InterruptStackFrame);
        fn do_isr_67(_: &mut InterruptStackFrame);
        fn do_isr_68(_: &mut InterruptStackFrame);
        fn do_isr_69(_: &mut InterruptStackFrame);
        fn do_isr_70(_: &mut InterruptStackFrame);
        fn do_isr_71(_: &mut InterruptStackFrame);
        fn do_isr_72(_: &mut InterruptStackFrame);
        fn do_isr_73(_: &mut InterruptStackFrame);
        fn do_isr_74(_: &mut InterruptStackFrame);
        fn do_isr_75(_: &mut InterruptStackFrame);
        fn do_isr_76(_: &mut InterruptStackFrame);
        fn do_isr_77(_: &mut InterruptStackFrame);
        fn do_isr_78(_: &mut InterruptStackFrame);
        fn do_isr_79(_: &mut InterruptStackFrame);
        fn do_isr_80(_: &mut InterruptStackFrame);
        fn do_isr_81(_: &mut InterruptStackFrame);
        fn do_isr_82(_: &mut InterruptStackFrame);
        fn do_isr_83(_: &mut InterruptStackFrame);
        fn do_isr_84(_: &mut InterruptStackFrame);
        fn do_isr_85(_: &mut InterruptStackFrame);
        fn do_isr_86(_: &mut InterruptStackFrame);
        fn do_isr_87(_: &mut InterruptStackFrame);
        fn do_isr_88(_: &mut InterruptStackFrame);
        fn do_isr_89(_: &mut InterruptStackFrame);
        fn do_isr_90(_: &mut InterruptStackFrame);
        fn do_isr_91(_: &mut InterruptStackFrame);
        fn do_isr_92(_: &mut InterruptStackFrame);
        fn do_isr_93(_: &mut InterruptStackFrame);
        fn do_isr_94(_: &mut InterruptStackFrame);
        fn do_isr_95(_: &mut InterruptStackFrame);
        fn do_isr_96(_: &mut InterruptStackFrame);
        fn do_isr_97(_: &mut InterruptStackFrame);
        fn do_isr_98(_: &mut InterruptStackFrame);
        fn do_isr_99(_: &mut InterruptStackFrame);
        fn do_isr_100(_: &mut InterruptStackFrame);
        fn do_isr_101(_: &mut InterruptStackFrame);
        fn do_isr_102(_: &mut InterruptStackFrame);
        fn do_isr_103(_: &mut InterruptStackFrame);
        fn do_isr_104(_: &mut InterruptStackFrame);
        fn do_isr_105(_: &mut InterruptStackFrame);
        fn do_isr_106(_: &mut InterruptStackFrame);
        fn do_isr_107(_: &mut InterruptStackFrame);
        fn do_isr_108(_: &mut InterruptStackFrame);
        fn do_isr_109(_: &mut InterruptStackFrame);
        fn do_isr_110(_: &mut InterruptStackFrame);
        fn do_isr_111(_: &mut InterruptStackFrame);
        fn do_isr_112(_: &mut InterruptStackFrame);
        fn do_isr_113(_: &mut InterruptStackFrame);
        fn do_isr_114(_: &mut InterruptStackFrame);
        fn do_isr_115(_: &mut InterruptStackFrame);
        fn do_isr_116(_: &mut InterruptStackFrame);
        fn do_isr_117(_: &mut InterruptStackFrame);
        fn do_isr_118(_: &mut InterruptStackFrame);
        fn do_isr_119(_: &mut InterruptStackFrame);
        fn do_isr_120(_: &mut InterruptStackFrame);
        fn do_isr_121(_: &mut InterruptStackFrame);
        fn do_isr_122(_: &mut InterruptStackFrame);
        fn do_isr_123(_: &mut InterruptStackFrame);
        fn do_isr_124(_: &mut InterruptStackFrame);
        fn do_isr_125(_: &mut InterruptStackFrame);
        fn do_isr_126(_: &mut InterruptStackFrame);
        fn do_isr_127(_: &mut InterruptStackFrame);
        fn do_isr_128(_: &mut InterruptStackFrame);
        fn do_isr_129(_: &mut InterruptStackFrame);
        fn do_isr_130(_: &mut InterruptStackFrame);
        fn do_isr_131(_: &mut InterruptStackFrame);
        fn do_isr_132(_: &mut InterruptStackFrame);
        fn do_isr_133(_: &mut InterruptStackFrame);
        fn do_isr_134(_: &mut InterruptStackFrame);
        fn do_isr_135(_: &mut InterruptStackFrame);
        fn do_isr_136(_: &mut InterruptStackFrame);
        fn do_isr_137(_: &mut InterruptStackFrame);
        fn do_isr_138(_: &mut InterruptStackFrame);
        fn do_isr_139(_: &mut InterruptStackFrame);
        fn do_isr_140(_: &mut InterruptStackFrame);
        fn do_isr_141(_: &mut InterruptStackFrame);
        fn do_isr_142(_: &mut InterruptStackFrame);
        fn do_isr_143(_: &mut InterruptStackFrame);
        fn do_isr_144(_: &mut InterruptStackFrame);
        fn do_isr_145(_: &mut InterruptStackFrame);
        fn do_isr_146(_: &mut InterruptStackFrame);
        fn do_isr_147(_: &mut InterruptStackFrame);
        fn do_isr_148(_: &mut InterruptStackFrame);
        fn do_isr_149(_: &mut InterruptStackFrame);
        fn do_isr_150(_: &mut InterruptStackFrame);
        fn do_isr_151(_: &mut InterruptStackFrame);
        fn do_isr_152(_: &mut InterruptStackFrame);
        fn do_isr_153(_: &mut InterruptStackFrame);
        fn do_isr_154(_: &mut InterruptStackFrame);
        fn do_isr_155(_: &mut InterruptStackFrame);
        fn do_isr_156(_: &mut InterruptStackFrame);
        fn do_isr_157(_: &mut InterruptStackFrame);
        fn do_isr_158(_: &mut InterruptStackFrame);
        fn do_isr_159(_: &mut InterruptStackFrame);
        fn do_isr_160(_: &mut InterruptStackFrame);
        fn do_isr_161(_: &mut InterruptStackFrame);
        fn do_isr_162(_: &mut InterruptStackFrame);
        fn do_isr_163(_: &mut InterruptStackFrame);
        fn do_isr_164(_: &mut InterruptStackFrame);
        fn do_isr_165(_: &mut InterruptStackFrame);
        fn do_isr_166(_: &mut InterruptStackFrame);
        fn do_isr_167(_: &mut InterruptStackFrame);
        fn do_isr_168(_: &mut InterruptStackFrame);
        fn do_isr_169(_: &mut InterruptStackFrame);
        fn do_isr_170(_: &mut InterruptStackFrame);
        fn do_isr_171(_: &mut InterruptStackFrame);
        fn do_isr_172(_: &mut InterruptStackFrame);
        fn do_isr_173(_: &mut InterruptStackFrame);
        fn do_isr_174(_: &mut InterruptStackFrame);
        fn do_isr_175(_: &mut InterruptStackFrame);
        fn do_isr_176(_: &mut InterruptStackFrame);
        fn do_isr_177(_: &mut InterruptStackFrame);
        fn do_isr_178(_: &mut InterruptStackFrame);
        fn do_isr_179(_: &mut InterruptStackFrame);
        fn do_isr_180(_: &mut InterruptStackFrame);
        fn do_isr_181(_: &mut InterruptStackFrame);
        fn do_isr_182(_: &mut InterruptStackFrame);
        fn do_isr_183(_: &mut InterruptStackFrame);
        fn do_isr_184(_: &mut InterruptStackFrame);
        fn do_isr_185(_: &mut InterruptStackFrame);
        fn do_isr_186(_: &mut InterruptStackFrame);
        fn do_isr_187(_: &mut InterruptStackFrame);
        fn do_isr_188(_: &mut InterruptStackFrame);
        fn do_isr_189(_: &mut InterruptStackFrame);
        fn do_isr_190(_: &mut InterruptStackFrame);
        fn do_isr_191(_: &mut InterruptStackFrame);
        fn do_isr_192(_: &mut InterruptStackFrame);
        fn do_isr_193(_: &mut InterruptStackFrame);
        fn do_isr_194(_: &mut InterruptStackFrame);
        fn do_isr_195(_: &mut InterruptStackFrame);
        fn do_isr_196(_: &mut InterruptStackFrame);
        fn do_isr_197(_: &mut InterruptStackFrame);
        fn do_isr_198(_: &mut InterruptStackFrame);
        fn do_isr_199(_: &mut InterruptStackFrame);
        fn do_isr_200(_: &mut InterruptStackFrame);
        fn do_isr_201(_: &mut InterruptStackFrame);
        fn do_isr_202(_: &mut InterruptStackFrame);
        fn do_isr_203(_: &mut InterruptStackFrame);
        fn do_isr_204(_: &mut InterruptStackFrame);
        fn do_isr_205(_: &mut InterruptStackFrame);
        fn do_isr_206(_: &mut InterruptStackFrame);
        fn do_isr_207(_: &mut InterruptStackFrame);
        fn do_isr_208(_: &mut InterruptStackFrame);
        fn do_isr_209(_: &mut InterruptStackFrame);
        fn do_isr_210(_: &mut InterruptStackFrame);
        fn do_isr_211(_: &mut InterruptStackFrame);
        fn do_isr_212(_: &mut InterruptStackFrame);
        fn do_isr_213(_: &mut InterruptStackFrame);
        fn do_isr_214(_: &mut InterruptStackFrame);
        fn do_isr_215(_: &mut InterruptStackFrame);
        fn do_isr_216(_: &mut InterruptStackFrame);
        fn do_isr_217(_: &mut InterruptStackFrame);
        fn do_isr_218(_: &mut InterruptStackFrame);
        fn do_isr_219(_: &mut InterruptStackFrame);
        fn do_isr_220(_: &mut InterruptStackFrame);
        fn do_isr_221(_: &mut InterruptStackFrame);
        fn do_isr_222(_: &mut InterruptStackFrame);
        fn do_isr_223(_: &mut InterruptStackFrame);
        fn do_isr_224(_: &mut InterruptStackFrame);
        fn do_isr_225(_: &mut InterruptStackFrame);
        fn do_isr_226(_: &mut InterruptStackFrame);
        fn do_isr_227(_: &mut InterruptStackFrame);
        fn do_isr_228(_: &mut InterruptStackFrame);
        fn do_isr_229(_: &mut InterruptStackFrame);
        fn do_isr_230(_: &mut InterruptStackFrame);
        fn do_isr_231(_: &mut InterruptStackFrame);
        fn do_isr_232(_: &mut InterruptStackFrame);
        fn do_isr_233(_: &mut InterruptStackFrame);
        fn do_isr_234(_: &mut InterruptStackFrame);
        fn do_isr_235(_: &mut InterruptStackFrame);
        fn do_isr_236(_: &mut InterruptStackFrame);
        fn do_isr_237(_: &mut InterruptStackFrame);
        fn do_isr_238(_: &mut InterruptStackFrame);
        fn do_isr_239(_: &mut InterruptStackFrame);
        fn do_isr_240(_: &mut InterruptStackFrame);
        fn do_isr_241(_: &mut InterruptStackFrame);
        fn do_isr_242(_: &mut InterruptStackFrame);
        fn do_isr_243(_: &mut InterruptStackFrame);
        fn do_isr_244(_: &mut InterruptStackFrame);
        fn do_isr_245(_: &mut InterruptStackFrame);
        fn do_isr_246(_: &mut InterruptStackFrame);
        fn do_isr_247(_: &mut InterruptStackFrame);
        fn do_isr_248(_: &mut InterruptStackFrame);
        fn do_isr_249(_: &mut InterruptStackFrame);
        fn do_isr_250(_: &mut InterruptStackFrame);
        fn do_isr_251(_: &mut InterruptStackFrame);
        fn do_isr_252(_: &mut InterruptStackFrame);
        fn do_isr_253(_: &mut InterruptStackFrame);
        fn do_isr_254(_: &mut InterruptStackFrame);
        fn do_isr_255(_: &mut InterruptStackFrame);
        fn invalid_opcode(_: &mut InterruptStackFrame);
        fn double_fault(_: &mut InterruptStackFrame, _: crate::x86_64::interrupt::MustbeZero) -> !;
        fn nmi(_: &mut InterruptStackFrame);
        fn general_protection_fault(_: &mut InterruptStackFrame, _: SegmentSelector);
        fn page_fault(_: &mut InterruptStackFrame, _: PFErrorCode);
        fn device_not_available(_: &mut InterruptStackFrame);
        fn simd_floating_point_exception(_: &mut InterruptStackFrame);
    }
    let idt = unsafe { &mut *core::ptr::addr_of_mut!(IDT) };
    idt.invalid_opcode.set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        invalid_opcode,
    );
    idt.nmi.set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        nmi,
    );
    idt.double_fault.set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        double_fault,
    );
    idt.device_not_available.set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        device_not_available,
    );
    idt.general_protection.set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        general_protection_fault,
    );
    idt.page_fault.set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        page_fault,
    );
    idt.simd_floating_point_exception.set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        simd_floating_point_exception,
    );
    idt.user_defined[0].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_32,
    );
    idt.user_defined[1].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_33,
    );
    idt.user_defined[2].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_34,
    );
    idt.user_defined[3].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_35,
    );
    idt.user_defined[4].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_36,
    );
    idt.user_defined[5].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_37,
    );
    idt.user_defined[6].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_38,
    );
    idt.user_defined[7].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_39,
    );
    idt.user_defined[8].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_40,
    );
    idt.user_defined[9].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_41,
    );
    idt.user_defined[10].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_42,
    );
    idt.user_defined[11].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_43,
    );
    idt.user_defined[12].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_44,
    );
    idt.user_defined[13].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_45,
    );
    idt.user_defined[14].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_46,
    );
    idt.user_defined[15].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_47,
    );
    idt.user_defined[16].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_48,
    );
    idt.user_defined[17].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_49,
    );
    idt.user_defined[18].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_50,
    );
    idt.user_defined[19].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_51,
    );
    idt.user_defined[20].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_52,
    );
    idt.user_defined[21].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_53,
    );
    idt.user_defined[22].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_54,
    );
    idt.user_defined[23].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_55,
    );
    idt.user_defined[24].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_56,
    );
    idt.user_defined[25].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_57,
    );
    idt.user_defined[26].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_58,
    );
    idt.user_defined[27].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_59,
    );
    idt.user_defined[28].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_60,
    );
    idt.user_defined[29].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_61,
    );
    idt.user_defined[30].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_62,
    );
    idt.user_defined[31].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_63,
    );
    idt.user_defined[32].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_64,
    );
    idt.user_defined[33].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_65,
    );
    idt.user_defined[34].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_66,
    );
    idt.user_defined[35].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_67,
    );
    idt.user_defined[36].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_68,
    );
    idt.user_defined[37].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_69,
    );
    idt.user_defined[38].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_70,
    );
    idt.user_defined[39].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_71,
    );
    idt.user_defined[40].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_72,
    );
    idt.user_defined[41].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_73,
    );
    idt.user_defined[42].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_74,
    );
    idt.user_defined[43].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_75,
    );
    idt.user_defined[44].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_76,
    );
    idt.user_defined[45].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_77,
    );
    idt.user_defined[46].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_78,
    );
    idt.user_defined[47].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_79,
    );
    idt.user_defined[48].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_80,
    );
    idt.user_defined[49].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_81,
    );
    idt.user_defined[50].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_82,
    );
    idt.user_defined[51].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_83,
    );
    idt.user_defined[52].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_84,
    );
    idt.user_defined[53].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_85,
    );
    idt.user_defined[54].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_86,
    );
    idt.user_defined[55].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_87,
    );
    idt.user_defined[56].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_88,
    );
    idt.user_defined[57].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_89,
    );
    idt.user_defined[58].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_90,
    );
    idt.user_defined[59].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_91,
    );
    idt.user_defined[60].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_92,
    );
    idt.user_defined[61].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_93,
    );
    idt.user_defined[62].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_94,
    );
    idt.user_defined[63].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_95,
    );
    idt.user_defined[64].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_96,
    );
    idt.user_defined[65].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_97,
    );
    idt.user_defined[66].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_98,
    );
    idt.user_defined[67].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_99,
    );
    idt.user_defined[68].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_100,
    );
    idt.user_defined[69].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_101,
    );
    idt.user_defined[70].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_102,
    );
    idt.user_defined[71].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_103,
    );
    idt.user_defined[72].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_104,
    );
    idt.user_defined[73].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_105,
    );
    idt.user_defined[74].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_106,
    );
    idt.user_defined[75].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_107,
    );
    idt.user_defined[76].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_108,
    );
    idt.user_defined[77].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_109,
    );
    idt.user_defined[78].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_110,
    );
    idt.user_defined[79].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_111,
    );
    idt.user_defined[80].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_112,
    );
    idt.user_defined[81].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_113,
    );
    idt.user_defined[82].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_114,
    );
    idt.user_defined[83].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_115,
    );
    idt.user_defined[84].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_116,
    );
    idt.user_defined[85].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_117,
    );
    idt.user_defined[86].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_118,
    );
    idt.user_defined[87].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_119,
    );
    idt.user_defined[88].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_120,
    );
    idt.user_defined[89].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_121,
    );
    idt.user_defined[90].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_122,
    );
    idt.user_defined[91].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_123,
    );
    idt.user_defined[92].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_124,
    );
    idt.user_defined[93].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_125,
    );
    idt.user_defined[94].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_126,
    );
    idt.user_defined[95].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_127,
    );
    idt.user_defined[96].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_128,
    );
    idt.user_defined[97].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_129,
    );
    idt.user_defined[98].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_130,
    );
    idt.user_defined[99].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_131,
    );
    idt.user_defined[100].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_132,
    );
    idt.user_defined[101].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_133,
    );
    idt.user_defined[102].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_134,
    );
    idt.user_defined[103].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_135,
    );
    idt.user_defined[104].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_136,
    );
    idt.user_defined[105].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_137,
    );
    idt.user_defined[106].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_138,
    );
    idt.user_defined[107].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_139,
    );
    idt.user_defined[108].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_140,
    );
    idt.user_defined[109].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_141,
    );
    idt.user_defined[110].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_142,
    );
    idt.user_defined[111].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_143,
    );
    idt.user_defined[112].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_144,
    );
    idt.user_defined[113].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_145,
    );
    idt.user_defined[114].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_146,
    );
    idt.user_defined[115].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_147,
    );
    idt.user_defined[116].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_148,
    );
    idt.user_defined[117].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_149,
    );
    idt.user_defined[118].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_150,
    );
    idt.user_defined[119].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_151,
    );
    idt.user_defined[120].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_152,
    );
    idt.user_defined[121].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_153,
    );
    idt.user_defined[122].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_154,
    );
    idt.user_defined[123].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_155,
    );
    idt.user_defined[124].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_156,
    );
    idt.user_defined[125].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_157,
    );
    idt.user_defined[126].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_158,
    );
    idt.user_defined[127].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_159,
    );
    idt.user_defined[128].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_160,
    );
    idt.user_defined[129].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_161,
    );
    idt.user_defined[130].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_162,
    );
    idt.user_defined[131].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_163,
    );
    idt.user_defined[132].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_164,
    );
    idt.user_defined[133].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_165,
    );
    idt.user_defined[134].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_166,
    );
    idt.user_defined[135].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_167,
    );
    idt.user_defined[136].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_168,
    );
    idt.user_defined[137].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_169,
    );
    idt.user_defined[138].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_170,
    );
    idt.user_defined[139].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_171,
    );
    idt.user_defined[140].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_172,
    );
    idt.user_defined[141].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_173,
    );
    idt.user_defined[142].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_174,
    );
    idt.user_defined[143].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_175,
    );
    idt.user_defined[144].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_176,
    );
    idt.user_defined[145].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_177,
    );
    idt.user_defined[146].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_178,
    );
    idt.user_defined[147].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_179,
    );
    idt.user_defined[148].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_180,
    );
    idt.user_defined[149].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_181,
    );
    idt.user_defined[150].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_182,
    );
    idt.user_defined[151].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_183,
    );
    idt.user_defined[152].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_184,
    );
    idt.user_defined[153].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_185,
    );
    idt.user_defined[154].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_186,
    );
    idt.user_defined[155].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_187,
    );
    idt.user_defined[156].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_188,
    );
    idt.user_defined[157].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_189,
    );
    idt.user_defined[158].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_190,
    );
    idt.user_defined[159].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_191,
    );
    idt.user_defined[160].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_192,
    );
    idt.user_defined[161].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_193,
    );
    idt.user_defined[162].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_194,
    );
    idt.user_defined[163].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_195,
    );
    idt.user_defined[164].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_196,
    );
    idt.user_defined[165].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_197,
    );
    idt.user_defined[166].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_198,
    );
    idt.user_defined[167].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_199,
    );
    idt.user_defined[168].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_200,
    );
    idt.user_defined[169].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_201,
    );
    idt.user_defined[170].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_202,
    );
    idt.user_defined[171].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_203,
    );
    idt.user_defined[172].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_204,
    );
    idt.user_defined[173].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_205,
    );
    idt.user_defined[174].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_206,
    );
    idt.user_defined[175].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_207,
    );
    idt.user_defined[176].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_208,
    );
    idt.user_defined[177].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_209,
    );
    idt.user_defined[178].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_210,
    );
    idt.user_defined[179].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_211,
    );
    idt.user_defined[180].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_212,
    );
    idt.user_defined[181].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_213,
    );
    idt.user_defined[182].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_214,
    );
    idt.user_defined[183].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_215,
    );
    idt.user_defined[184].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_216,
    );
    idt.user_defined[185].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_217,
    );
    idt.user_defined[186].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_218,
    );
    idt.user_defined[187].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_219,
    );
    idt.user_defined[188].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_220,
    );
    idt.user_defined[189].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_221,
    );
    idt.user_defined[190].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_222,
    );
    idt.user_defined[191].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_223,
    );
    idt.user_defined[192].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_224,
    );
    idt.user_defined[193].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_225,
    );
    idt.user_defined[194].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_226,
    );
    idt.user_defined[195].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_227,
    );
    idt.user_defined[196].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_228,
    );
    idt.user_defined[197].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_229,
    );
    idt.user_defined[198].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_230,
    );
    idt.user_defined[199].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_231,
    );
    idt.user_defined[200].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_232,
    );
    idt.user_defined[201].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_233,
    );
    idt.user_defined[202].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_234,
    );
    idt.user_defined[203].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_235,
    );
    idt.user_defined[204].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_236,
    );
    idt.user_defined[205].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_237,
    );
    idt.user_defined[206].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_238,
    );
    idt.user_defined[207].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_239,
    );
    idt.user_defined[208].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_240,
    );
    idt.user_defined[209].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_241,
    );
    idt.user_defined[210].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_242,
    );
    idt.user_defined[211].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_243,
    );
    idt.user_defined[212].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_244,
    );
    idt.user_defined[213].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_245,
    );
    idt.user_defined[214].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_246,
    );
    idt.user_defined[215].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_247,
    );
    idt.user_defined[216].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_248,
    );
    idt.user_defined[217].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_249,
    );
    idt.user_defined[218].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_250,
    );
    idt.user_defined[219].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_251,
    );
    idt.user_defined[220].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_252,
    );
    idt.user_defined[221].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_253,
    );
    idt.user_defined[222].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_254,
    );
    idt.user_defined[223].set(
        Segment::KernelCode.into_selector(),
        ExceptionType::Interrupt,
        do_isr_255,
    );
}
