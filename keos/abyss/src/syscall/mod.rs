//! System Call

mod entry;

use crate::x86_64::{Rflags, msr::Msr, segmentation::Segment};

unsafe extern "C" {
    fn arch_syscall_entry();
}

pub fn syscall_entry_register() {
    unsafe {
        Msr::<0xC000_0081>::write(
            ((Segment::UserCode.into_selector().pack() as u64).wrapping_sub(0x10) << 48)
                | ((Segment::KernelCode.into_selector().pack() as u64) << 32),
        );

        Msr::<0xC000_0082>::write(arch_syscall_entry as usize as u64);

        Msr::<0xC000_0084>::write(
            (Rflags::IF | Rflags::TF | Rflags::DF | Rflags::IOPL0 | Rflags::AC | Rflags::NT).bits(),
        );
    }
}
