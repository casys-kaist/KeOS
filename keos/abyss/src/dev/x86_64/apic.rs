//! Advanced Programmable Interrupt Controller (APIC) driver.
//!
//! This implements X2Apic mode.
use crate::dev::DeviceError;
use crate::x86_64::{msr::Msr, pio::Pio};
use core::convert::TryFrom;

enum MapDest {
    Master(u8),
    Slave(u8),
}

impl TryFrom<u8> for MapDest {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 32 {
            Err(())
        } else if value < 40 {
            Ok(Self::Master(value - 32))
        } else if value < 48u8 {
            Ok(Self::Slave(value - 40))
        } else {
            Err(())
        }
    }
}

/// 8259A interrupt controller
pub struct _8259A;

impl _8259A {
    const MASK: u16 = 0b1111_1111_1111_1111;

    pub(crate) fn init() {
        // mask all of 8259A
        Pio::new(0x21).write_u8(0xff);
        Pio::new(0xa1).write_u8(0xff);

        // ICW1: select 8259A-1 init
        Pio::new(0x20).write_u8(0x11);
        // ICW2: 8259A-1 IR0-7 mapped to 32.
        Pio::new(0x21).write_u8(32);
        // ICW3: 8259A-1 (the master) has a slave on IR2
        Pio::new(0x21).write_u8(1 << 2);
        // ICW4: master does Auto EOI
        Pio::new(0x21).write_u8(3);

        // Setup slave
        // ICW1: select 8259A-2 init
        Pio::new(0xa0).write_u8(0x11);
        // ICW2: 8259A-2 IR0-7 mapped to 40.
        Pio::new(0xa1).write_u8(40);
        // ICW3: 8259A-2 is a slave on master's IR2
        Pio::new(0xa1).write_u8(2);
        // ICW4: slave's support for AEOI in flat mode is to be investigated.
        Pio::new(0xa1).write_u8(1);

        // clear specific mask.
        Pio::new(0x20).write_u8(0x68);
        // read IRR by default.
        Pio::new(0x20).write_u8(0x0a);

        Pio::new(0xa0).write_u8(0x68);
        Pio::new(0xa0).write_u8(0x0a);

        // restore master IRQ mask.
        Pio::new(0x21).write_u8(Self::MASK as u8);
        // restore slave IRQ mask.
        Pio::new(0xa1).write_u8((Self::MASK >> 8) as u8);
    }

    #[allow(dead_code)]
    pub(crate) fn enable(ev: u8) -> Result<(), ()> {
        MapDest::try_from(ev).map(|dest| {
            let (port, mask) = match dest {
                MapDest::Master(n) => (Pio::new(0x21), 1 << n),
                MapDest::Slave(n) => (Pio::new(0xa1), 1 << n),
            };
            port.write_u8(port.read_u8() & !mask)
        })
    }

    #[allow(dead_code)]
    pub(crate) fn disable(ev: u8) -> Result<(), ()> {
        MapDest::try_from(ev).map(|dest| {
            let (port, mask) = match dest {
                MapDest::Master(n) => (Pio::new(0x21), 1 << n),
                MapDest::Slave(n) => (Pio::new(0xa1), 1 << n),
            };
            port.write_u8(port.read_u8() | mask)
        })
    }
}

pub unsafe fn init(core_id: usize) -> Result<(), DeviceError> {
    unsafe {
        if core::arch::x86_64::__cpuid(1).ecx & (1 << 21) != 0 {
            // Enable the x2 apic
            let apic_base = Msr::<0x1b>::read();
            Msr::<0x1b>::write(apic_base | (1 << 10));
            // Enable local apic and set susprious irq vector.
            // IRQ_SUSPRIOUS = 0xff;
            // SIV
            Msr::<0x80f>::write(0x100 | 0xff);
            // TP
            Msr::<0x808>::write((Msr::<0x808>::read() & 0xff) | 0x10);
            // lint1 = MASK | NMI
            // Msr::<0x836>::write(0x10000 | 0x400);
            if core_id == 0 {
                // lint0
                Msr::<0x835>::write(0x700);
                _8259A::init();
            } else {
                // lint0
                // MASK | ExtInt
                Msr::<0x835>::write(0x10000 | 0x700);
            }
            Ok(())
        } else {
            Err(DeviceError("X2Apic is not supported."))
        }
    }
}

pub fn eoi() {
    unsafe {
        Msr::<0x80b>::write(0);
    }
}

pub enum IPIDest {
    Cpu(usize),
    _Self,
    AllIncludingSelf,
    AllExcludingSelf,
}

pub enum Mode {
    Fixed(u8),
    Smi,
    Nmi,
    Init,
    StartUp(u8),
}

pub unsafe fn send_ipi(dest: IPIDest, mode: Mode) {
    unsafe {
        Msr::<0x830>::write(
            match dest {
                IPIDest::Cpu(cpu) => (cpu as u64) << 32,
                IPIDest::_Self => 0b01 << 18,
                IPIDest::AllIncludingSelf => 0b10 << 18,
                IPIDest::AllExcludingSelf => 0b11 << 18,
            } | match mode {
                Mode::Fixed(v) => v as u64,
                Mode::Smi => 0b010 << 8,
                Mode::Nmi => 0b100 << 8,
                Mode::Init => 0b101 << 8,
                Mode::StartUp(v) => (0b110 << 8) | (v as u64),
            } | 0x4000,
        );
    }
}
