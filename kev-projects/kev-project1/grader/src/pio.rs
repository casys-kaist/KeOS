use core::arch::global_asm;

// print 'Hello pio\n' and exit.
global_asm!(
    "pio_print_start:",
    "in al, 3",
    // Hello\n
    "mov dx, 0x3f8",
    "mov al, 0x48",
    "out dx, al",
    "mov al, 0x65",
    "out dx, al",
    "mov al, 0x6c",
    "out dx, al",
    "mov al, 0x6c",
    "out dx, al",
    "mov al, 0x6f",
    "out dx, al",
    "mov al, 0x20",
    "out dx, al",
    "mov al, 0x70",
    "out dx, al",
    "mov al, 0x69",
    "out dx, al",
    "mov al, 0x6f",
    "out dx, al",
    "mov al, 0x20",
    "out dx, al",
    // hcall_exit(0);
    "mov rdi, 0",
    "mov rax, 0",
    "vmcall",
    "pio_print_end:",
);
pub fn pio_print() {
    super::run_vm::<0>(unsafe {
        unsafe extern "C" {
            static pio_print_start: u8;
            static pio_print_end: u8;
        }
        core::slice::from_raw_parts(
            &pio_print_start as *const u8,
            &pio_print_end as *const _ as usize - &pio_print_start as *const _ as usize,
        )
    });
}

// Test for out/in (e)a(x|l), dx instructions
// Check PioQueueHandler in pio.rs that represents queuing operations.
global_asm!(
    "pio_dx_port_start:",
    // out dx, (e)a(x|l)
    "mov dx, 0xbb",
    "mov al, 0x11",
    "out dx, al", // Out_DX_AL
    "mov ax, 0x2222",
    "out dx, ax", // Out_DX_AX
    "mov eax, 0x33333333",
    "out dx, eax", // Out_DX_EAX
    // in (e)a(x|l), dx
    "xor al, al",
    "in al, dx", // In_AL_DX
    "cmp al, 0x11",
    "jne pio_dx_port_failed",
    "xor ax, ax",
    "in ax, dx", // In_AX_DX
    "cmp ax, 0x2222",
    "jne pio_dx_port_failed",
    "xor eax, eax",
    "in eax, dx", // In_EAX_DX
    "cmp eax, 0x33333333",
    "jne pio_dx_port_failed",
    // hcall_exit(0);
    "mov rdi, 0",
    "mov rax, 0",
    "vmcall",
    "pio_dx_port_failed:",
    // hcall_exit(1); if failed
    "mov rdi, 1",
    "mov rax, 0",
    "vmcall",
    "pio_dx_port_end:",
);
pub fn pio_dx_port() {
    super::run_vm::<0>(unsafe {
        unsafe extern "C" {
            static pio_dx_port_start: u8;
            static pio_dx_port_end: u8;
        }
        core::slice::from_raw_parts(
            &pio_dx_port_start as *const u8,
            &pio_dx_port_end as *const _ as usize - &pio_dx_port_start as *const _ as usize,
        )
    });
}

// Test for out/in (e)a(x|l), imm8 instructions
// Check PioQueueHandler in pio.rs that represents queuing operations.
global_asm!(
    "pio_imm8_port_start:",
    // out imm8, (e)a(x|l)
    "mov al, 0x44",
    "out 0xbb, al", // Out_imm8_AL
    "mov ax, 0x5555",
    "out 0xbb, ax", // Out_imm8_AX
    "mov eax, 0x66666666",
    "out 0xbb, eax", // Out_imm8_EAX
    // in (e)a(x|l), imm8
    "xor al, al",
    "in al, 0xbb", // In_AL_Imm8
    "cmp al, 0x44",
    "jne pio_imm8_port_failed",
    "xor ax, ax",
    "in ax, 0xbb", // In_AX_Imm8
    "cmp ax, 0x5555",
    "jne pio_imm8_port_failed",
    "xor eax, eax",
    "in eax, 0xbb", // In_EAX_Imm8
    "cmp eax, 0x66666666",
    "jne pio_imm8_port_failed",
    // hcall_exit(0);
    "mov rdi, 0",
    "mov rax, 0",
    "vmcall",
    "pio_imm8_port_failed:",
    // hcall_exit(1); if failed
    "mov rdi, 1",
    "mov rax, 0",
    "vmcall",
    "pio_imm8_port_end:",
);
pub fn pio_imm8_port() {
    super::run_vm::<0>(unsafe {
        unsafe extern "C" {
            static pio_imm8_port_start: u8;
            static pio_imm8_port_end: u8;
        }
        core::slice::from_raw_parts(
            &pio_imm8_port_start as *const u8,
            &pio_imm8_port_end as *const _ as usize - &pio_imm8_port_start as *const _ as usize,
        )
    });
}

// Test for outs(b|w|d), ins(b|w|d) instructions
// Check PioQueueHandler in pio.rs that represents queuing operations.
global_asm!(
    "pio_mem_start:",
    "mov dx, 0xbb",
    // outs(b|w|d)
    "lea rsi, [rip + pio_mem_byte]",
    "outsb", // Outsb_DX_m8
    "lea rsi, [rip + pio_mem_word]",
    "outsw", // Outsw_DX_m16
    "lea rsi, [rip + pio_mem_dword]",
    "outsd", // Outsd_DX_m32
    // ins(b|w|d)
    // gva of a writable region page
    "mov rdi, 0x2000",
    "cld",
    "mov byte ptr [rdi], 0",
    "insb", // Insb_m8_DX
    "cmp byte ptr [rdi - 1], 0x77",
    "jne pio_mem_failed",
    "mov word ptr [rdi], 0",
    "insw", // Insw_m16_DX
    "cmp word ptr [rdi - 2], 0x8888",
    "jne pio_mem_failed",
    "mov dword ptr [rdi], 0",
    "insd", // Insd_m32_DX
    "cmp dword ptr [rdi - 4], 0x99999999",
    "jne pio_mem_failed",
    // hcall_exit(0);
    "mov rdi, 0",
    "mov rax, 0",
    "vmcall",
    "pio_mem_failed:",
    // hcall_exit(1); if failed
    "mov rdi, 1",
    "mov rax, 0",
    "vmcall",
    "pio_mem_byte:",
    ".byte 0x77",
    "pio_mem_word:",
    ".2byte 0x8888",
    "pio_mem_dword:",
    ".4byte 0x99999999",
    "pio_mem_end:",
);
pub fn pio_mem() {
    super::run_vm::<0>(unsafe {
        unsafe extern "C" {
            static pio_mem_start: u8;
            static pio_mem_end: u8;
        }
        core::slice::from_raw_parts(
            &pio_mem_start as *const u8,
            &pio_mem_end as *const _ as usize - &pio_mem_start as *const _ as usize,
        )
    });
}
