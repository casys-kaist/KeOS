use core::arch::global_asm;

// Test for msr
global_asm!(
    "msr_start:",
    "mov rcx, 0xabc",
    "mov rdx, 0xFFFFFFFF11112222",
    "mov rax, 0xFFFFFFFF33334444",
    "wrmsr",
    "mov rdx, 0",
    "mov rax, 0",
    "rdmsr",
    "cmp rdx, 0x11112222",
    "jne msr_failed",
    "cmp rax, 0x33334444",
    "jne msr_failed",
    // hcall_exit(0);
    "mov rdi, 0",
    "mov rax, 0",
    "vmcall",
    "msr_failed:",
    "mov rdi, 1",
    "mov rax, 0",
    "vmcall",
    "msr_end:",
);
pub fn msr() {
    super::run_vm::<0>(unsafe {
        unsafe extern "C" {
            static msr_start: u8;
            static msr_end: u8;
        }
        core::slice::from_raw_parts(
            &msr_start as *const u8,
            &msr_end as *const _ as usize - &msr_start as *const _ as usize,
        )
    });
}
