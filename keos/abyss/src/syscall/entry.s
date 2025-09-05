.section .text
.globl arch_syscall_entry
.type arch_syscall_entry, @function

arch_syscall_entry:
    cld
    mov gs:[8], rsp              // Put RSP
    mov gs:[16], r14             // Backup R14

    mov r14, rcx
    shr r14, 48
    cmp r14, 0
    jz .__if_branch__user

.__if_branch__kern:
    // push the states from RSP.
    mov r14, 0x10             /* ss */
    push r14
    push QWORD PTR gs:[8]     /* original rsp */
    push r11                  /* rflags */
    mov r14, 0x08             /* cs */
jmp .__if_branch__done
    
.__if_branch__user:
    mov r14, gs:[0]              // Current CPU's TSS address
    mov rsp, [r14 + 4]           // Now we are in kernel stack
    /* Build a fake interrupt frame, stack grows down */
    mov r14, 0x1b             /* ss */
    push r14
    push QWORD PTR gs:[8]     /* userspace rsp */
    push r11                  /* rflags */
    mov r14, 0x23             /* cs */

.__if_branch__done:
    sti                       /* Now, we can use kernel stack. Allow interrupt here. */
    push r14
    push rcx                  /* rip */
    
    /* Bring back backed up registers*/
    mov r14, gs:[16]

    /* Now push general-purpose registers */
    sub rsp, 128
    // error code = 0
    mov DWORD PTR [rsp + 0x78], 0
    mov DWORD PTR [rsp + 0x7C], 0
    
    mov [rsp + 0x70], rax
    mov [rsp + 0x68], rbx
    mov DWORD PTR [rsp + 0x64], 0       /* rcx is caller-saved and already overwritten by CPU */
    mov DWORD PTR [rsp + 0x60], 0       /* rcx is caller-saved and already overwritten by CPU */
    mov [rsp + 0x58], rdx
    mov [rsp + 0x50], rbp
    mov [rsp + 0x48], rdi
    mov [rsp + 0x40], rsi
    mov [rsp + 0x38], r8
    mov [rsp + 0x30], r9
    mov [rsp + 0x28], r10
    mov DWORD PTR [rsp + 0x20], 0        /* r11 is caller-saved and already overwritten by CPU */
    mov DWORD PTR [rsp + 0x24], 0        /* r11 is caller-saved and already overwritten by CPU */
    mov [rsp + 0x18], r12
    mov [rsp + 0x10], r13
    mov [rsp + 0x8], r14
    mov [rsp], r15
    mov rsi, [rsp + 0x88]
    mov gs:[32], rsp
    mov rdi, rsp

    call do_handle_syscall

    mov DWORD PTR gs:[32], 0
    mov DWORD PTR gs:[36], 0
    mov rax, [rsp + 0x70]
    mov rbx, [rsp + 0x68]
    mov rcx, [rsp + 0x60]
    mov rdx, [rsp + 0x58]
    mov rbp, [rsp + 0x50]
    mov rdi, [rsp + 0x48]
    mov rsi, [rsp + 0x40]
    mov r8, [rsp + 0x38]
    mov r9, [rsp + 0x30]
    mov r10, [rsp + 0x28]
    mov r11, [rsp + 0x20]
    mov r12, [rsp + 0x18]
    mov r13, [rsp + 0x10]
    mov r14, [rsp + 0x8]
    mov r15, [rsp]
    add rsp, 128
    iretq