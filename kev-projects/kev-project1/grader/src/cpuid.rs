use core::arch::global_asm;
use kev::vm::VmBuilder;
use kev_project1::no_ept_vm::NoEptVmState;

// Get vendor from this core and exit.
global_asm!(
    "cpuid_leaf_0_start:",
    "mov rax, 0x0",
    "cpuid",
    "cmp rbx, 0x756e6547",
    "jne cpuid_leaf_0_failed",
    "mov rdi, 0",
    "mov rax, 0",
    "vmcall",
    "cpuid_leaf_0_failed:",
    "mov rdi, 1",
    "mov rax, 0",
    "vmcall",
    "cpuid_leaf_0_end:"
);
pub fn cpuid_leaf_0() {
    super::run_vm::<0>(unsafe {
        unsafe extern "C" {
            static cpuid_leaf_0_start: u8;
            static cpuid_leaf_0_end: u8;
        }
        core::slice::from_raw_parts(
            &cpuid_leaf_0_start as *const u8,
            &cpuid_leaf_0_end as *const _ as usize - &cpuid_leaf_0_start as *const _ as usize,
        )
    })
}

// Check the current virtual core id repeatedly and exit.
global_asm!(
    "cpuid_leaf_1_start:",
    "mov r8, 0x100",
    "l:",
    "mov rax, 0x1",
    "cpuid",
    "shr ebx, 24",
    "and ebx, 0xFF",
    "cmp ebx, 0xba",
    "jne cpuid_leaf_1_failed",
    "dec r8",
    "jnz l",
    "mov rdi, 0",
    "mov rax, 0",
    "vmcall",
    "cpuid_leaf_1_failed:",
    "mov rdi, 1",
    "mov rax, 0",
    "vmcall",
    "cpuid_leaf_1_end:"
);
pub fn cpuid_leaf_1() {
    let vm = VmBuilder::new(
        NoEptVmState::new(unsafe {
            unsafe extern "C" {
                static cpuid_leaf_1_start: u8;
                static cpuid_leaf_1_end: u8;
            }
            core::slice::from_raw_parts(
                &cpuid_leaf_1_start as *const u8,
                &cpuid_leaf_1_end as *const _ as usize - &cpuid_leaf_1_start as *const _ as usize,
            )
        }),
        1,
    )
    .expect("Failed to create vmbuilder.")
    .finalize()
    .expect("Failed to create vm.");
    let mut guard = vm.vcpu(0).unwrap().lock();
    guard.vcpu_id = 0xba;
    guard.unlock();
    vm.start_bsp().expect("Failed to start bsp.");
    assert_eq!(vm.join(), 0);
}
