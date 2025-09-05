// Copyright 2025 Computer Architecture and Systems Lab
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

.section .text.init
.code16

.global ap_trampoline
ap_trampoline:
    cli
    cld
    ljmp    0, 0x8010
    .align 16

_L8010:
    xor ax, ax
    mov ds, ax
    lgdt [0x8118]
    mov eax, cr0
    or eax, 1                    # CR0_PE
    mov cr0, eax
    ljmp 8, 0x8040
    .align 0x20
    .code32
_L8040:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    mov eax, cr4
    or eax, 0x00000020  # CR4_PAE
    and eax, 0xFFFEFFFF # ~CR4_FSGSBASE
    mov cr4, eax

    mov eax, [0x8148] # Load boot pml4e.
    mov cr3, eax

    # Enable the long mode
    mov ecx, 0xC0000080     # EFER_MSR
    rdmsr
    or eax, (1 << 8)        # EFER_LME
    wrmsr

    # Enable the paging
    mov eax, cr0
    or eax, (1 << 31)       # CR0_PG
    mov cr0, eax

    # Load gdt64
    lgdt [0x8138]
    mov esp, 0x8170
    push 0x8
    push 0x80a0
    retf

.align 0x20
.code64
_L80A0:
    # get LAPIC ID.
    mov eax, 1
    cpuid
    shr ebx, 24
    mov rdi, rbx
    # call secondary_init_kernel
    mov rax, [0x8150]
    jmp rax
    ud2


.align 0x100
_L8100_GDT_table:
    .quad 0;                    # NULL SEGMENT
    .quad 0xCF9A000000FFFF;     # CODE SEGMENT
    .quad 0xCF92000000FFFF;     # DATA SEGMENT

_L8118_GDT_value:
    .word 0x17
    .long 0x8100

.p2align 2
_L8120_GDT64_TABLE:
  .quad 0                   # NULL SEGMENT
  .quad 0x00af9a000000ffff  # CODE SEGMENT64
  .quad 0x00cf92000000ffff  # DATA SEGMENT64

_L8138_GDT64_value:
  .word 0x17
  .quad 0x8120

.align 8
.global boot_pml4e
boot_pml4e: // L8148
    .long 0, 0
_L8150_SECONDARY_KERNEL_INIT_MARKER:
    .quad start
_L8158_RETF_STACK_END:
    .quad 0, 0, 0
_L8170_RETF_STACK:

.global ap_trampoline_end
ap_trampoline_end:
