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

/* Multiboot2 Header */
.section .multiboot_header
header_start:
    .long 0xe85250d6                // Magic number
    .long 0                         // Flags
    .long header_end - header_start // Size of the Header
    .long 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start)) // checksum

    // fb
    .short 5
    .short 0
    .long 20
    .long 0
    .long 0
    .long 0
    .long 0

    // end tag
    .short 0
    .short 0
    .long 8
header_end:

.section .bootstrap, "awx"
.code32

.global bootstrap_start
bootstrap_start:
  mov esi, ebx
# Start bootup.
verify_cpu:
  pushf
  pop eax
  // Bit 21. Able to use CPUID.
  or eax, (1 << 21)
  push eax
  popf

  xor eax, eax           # cpuid 1 valid?
  cpuid
  cmp eax, 1
  jb no_long_mode        # cpuid 1 not valid.
  mov eax, 0x80000001
  cpuid
  test edx, (1 << 29)    # Check LM bit
  jz no_long_mode

  mov eax, cr4
  or eax, 0x00000020  # CR4_PAE
  and eax, 0xFFFEFFFF # ~CR4_FSGSBASE
  mov cr4, eax

# Now, setup the page table which cover 4GB space.
setup_pt:
  # setup the pdpts
  # PML4[0] = pdpt1, PML4[512] = pdpt2
  lea edi, [boot_page_table - 0xffffff0000000000]
  lea ebx, [boot_pdpt1 - 0xffffff0000000000]
  lea ecx, [boot_pdpt2 - 0xffffff0000000000]
  or ebx, 0x3            # PTE_P | PTE_W
  or ecx, 0x3            # PTE_P | PTE_W
  mov [edi], ebx
  mov [edi + 0xff0], ecx

  # setup the pdpes
  lea ebx, [boot_pdpt1 - 0xffffff0000000000]
  lea ecx, [boot_pdpt2 - 0xffffff0000000000]

  # pdpt1[0] = pde1, pdpt2[0] = pde1
  lea edi, [boot_pde1 - 0xffffff0000000000]
  or edi, 0x3            # PTE_P | PTE_W
  mov [ebx], edi
  mov [ecx], edi

  # pdpt1[1] = pde2, pdpt2[1] = pde2
  lea edi, [boot_pde2 - 0xffffff0000000000]
  or edi, 0x3            # PTE_P | PTE_W
  mov [ebx + 0x8], edi
  mov [ecx + 0x8], edi

  # pdpt1[2] = pde3, pdpt2[2] = pde3
  lea edi, [boot_pde3 - 0xffffff0000000000]
  or edi, 0x3            # PTE_P | PTE_W
  mov [ebx + 0x10], edi
  mov [ecx + 0x10], edi

  # pdpt1[3] = pde4, pdpt2[3] = pde4
  lea edi, [boot_pde4 - 0xffffff0000000000]
  or edi, 0x3            # PTE_P | PTE_W
  mov [ebx + 0x18], edi
  mov [ecx + 0x18], edi

  # pdpt1[4] = pde5, pdpt2[4] = pde5
  lea edi, [boot_pde5 - 0xffffff0000000000]
  or edi, 0x3            # PTE_P | PTE_W
  mov [ebx + 0x20], edi
  mov [ecx + 0x20], edi

  # pdpt1[5] = pde6, pdpt2[5] = pde6
  lea edi, [boot_pde6 - 0xffffff0000000000]
  or edi, 0x3            # PTE_P | PTE_W
  mov [ebx + 0x28], edi
  mov [ecx + 0x28], edi

  # pdpt1[6] = pde7, pdpt2[6] = pde7
  lea edi, [boot_pde7 - 0xffffff0000000000]
  or edi, 0x3            # PTE_P | PTE_W
  mov [ebx + 0x30], edi
  mov [ecx + 0x30], edi

  # pdpt1[7] = pde7, pdpt2[7] = pde7
  lea edi, [boot_pde8 - 0xffffff0000000000]
  or edi, 0x3            # PTE_P | PTE_W
  mov [ebx + 0x38], edi
  mov [ecx + 0x38], edi

  # setup the pdes with PTE_MBZ
  mov ecx, 512
  lea ebx, [boot_pde1 - 0xffffff0000000000]
  mov eax, 0x183         # PTE_P | PTE_W | PTE_MBZ
  call fill_pde

  mov ecx, 512
  lea ebx, [boot_pde2 - 0xffffff0000000000]
  mov eax, 0x40000183         # PTE_P | PTE_W | PTE_MBZ
  call fill_pde

  mov ecx, 512
  lea ebx, [boot_pde3 - 0xffffff0000000000]
  mov eax, 0x80000183         # PTE_P | PTE_W | PTE_MBZ
  call fill_pde

  mov ecx, 512
  lea ebx, [boot_pde4 - 0xffffff0000000000]
  mov eax, 0xc0000183         # PTE_P | PTE_W | PTE_MBZ
  call fill_pde

jump_long_mode:
  # load cr3
  lea eax, [boot_page_table - 0xffffff0000000000]
  mov cr3, eax

  # Enable the long mode
  mov ecx, 0xC0000080     # EFER_MSR
  rdmsr
  or eax, (1 << 8)        # EFER_LME
  wrmsr

  # Enable the paging
  mov eax, cr0
  or eax, (1 << 31) | (1 << 5)       # CR0_PE
  mov cr0, eax

  # Jump to the long mode
  lgdt [gdt_desc64 - 0xffffff0000000000]
  lea eax, [bootstrap_64 - 0xffffff0000000000]
  push 0x8
  push eax
  retf

# helpers
fill_pde:
  mov [ebx], eax
  add ebx, 8
  add eax, 0x200000
  dec ecx
  cmp ecx, 0
  jne fill_pde
  ret

no_long_mode:
  jmp no_long_mode

.p2align 2
// Debug purpose
debug_fb:
  .long 0
gdt64:
  .quad 0                   # NULL SEGMENT
  .quad 0x00af9a000000ffff  # CODE SEGMENT64
  .quad 0x00cf92000000ffff  # DATA SEGMENT64
gdt_desc64:
  .word 0x17
  .quad gdt64 - 0xffffff0000000000

.p2align 12
.globl boot_page_table
.globl boot_pdpt1
.globl boot_pdpt2
.globl boot_pde1
.globl boot_pde2
.globl boot_pde3
.globl boot_pde4
.globl boot_pde5
.globl boot_pde6
.globl boot_pde7
.globl boot_pde8
.globl IDLE_STACK

boot_page_table:
  .space  0x1000
boot_pdpt1:
  .space  0x1000
boot_pdpt2:
  .space  0x1000
boot_pde1:
  .space  0x1000
boot_pde2:
  .space  0x1000
boot_pde3:
  .space  0x1000
boot_pde4:
  .space  0x1000
boot_pde5:
  .space  0x1000
boot_pde6:
  .space  0x1000
boot_pde7:
  .space  0x1000
boot_pde8:
  .space  0x1000
_boot_stack_top:
  .space  0x1000
_boot_stack_bottom:

// We can have up to 8 cores.
IDLE_STACK:
  .space 0x800000


.code64
.extern kernel_init
# helpers
fill_pde2:
  mov [rbx], rax
  add rbx, 8
  add rax, 0x200000
  dec rcx
  cmp rcx, 0
  jne fill_pde2
  ret

bootstrap_64:
  # Now we are in 64bit world!
  mov rcx, 512
  lea rbx, [boot_pde5 - 0xffffff0000000000]
  mov rax, 0x100000183         # PTE_P | PTE_W | PTE_MBZ
  call fill_pde2
  mov rcx, 512
  lea rbx, [boot_pde6 - 0xffffff0000000000]
  mov rax, 0x140000183         # PTE_P | PTE_W | PTE_MBZ
  call fill_pde2
  mov rcx, 512
  lea rbx, [boot_pde7 - 0xffffff0000000000]
  mov rax, 0x180000183         # PTE_P | PTE_W | PTE_MBZ
  call fill_pde2
  mov rcx, 512
  lea rbx, [boot_pde8 - 0xffffff0000000000]
  mov rax, 0x1c0000183         # PTE_P | PTE_W | PTE_MBZ
  call fill_pde2

guest_start:
  # rsi = mbinfo
  mov ax, 0x10
  mov ds, ax
  mov ss, ax
  mov fs, ax
  mov gs, ax
  mov es, ax
  movabs rax, 0xffffff0000000000
  add rsi, rax

  mov eax, 1
  cpuid
  shr ebx, 24
  mov rdi, rbx # rdi contains mpid

  lea rax, [kernel_init_ptr - 0xffffff0000000000]
  mov rax, [rax]
  jmp rax

kernel_init_ptr:
  .quad start