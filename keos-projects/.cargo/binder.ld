ENTRY(bootstrap_start)
OUTPUT_FORMAT(elf64-x86-64)

KERNEL_BASE = 0xffffff0000000000;

SECTIONS {
  . = KERNEL_BASE;
  . += 1M;
  PROVIDE(__kernel_start = .);
  /* After early init is done, we can reclaim this memory */
  .init : AT(ADDR(.init) - KERNEL_BASE) {
    PROVIDE(__init_start = .);
    KEEP(*(.multiboot_header))
    *(.bootstrap .bootstrap.*)

    KEEP(*(.text.init))
    *(.data.init)
    . = ALIGN(0x1000);
    PROVIDE(__init_end = .);
  }

  .percpu : AT(ADDR(.percpu) - KERNEL_BASE) {
      PROVIDE(__per_cpu_start = .);
      /* 8 bytes for pointing itself */
      /* 8 bytes for saving user rsp */
      . += 16;
      *(.percpu.tss)
      *(.percpu .percpu.*)
      . = ALIGN(0x1000);
      PROVIDE(__per_cpu_end = .);
  }

  .text : AT(ADDR(.text) - KERNEL_BASE) {
      PROVIDE(__text_start = .);
      *(.text .text.* .gnu.linkonce.t*)
      . = ALIGN(0x1000);
      PROVIDE(__text_end = .);
  }

  PROVIDE(__rodata_start = .);
  .rodata : AT(ADDR(.rodata) - KERNEL_BASE) {
      *(.rodata .rodata.* .gnu.linkonce.r.*)
  }
  .eh_frame_hdr : AT(ADDR(.eh_frame_hdr) - KERNEL_BASE) {
      PROVIDE(__eh_frame_hdr_start = .);
      *(.eh_frame_hdr)
  }
  .eh_frame : AT(ADDR(.eh_frame) - KERNEL_BASE) {
      *(.eh_frame)
      *(.gcc_except_table .gcc_except_table.*)
      PROVIDE(__eh_frame_end = .);
  }
  . = ALIGN(0x1000);
  PROVIDE(__rodata_end = .);

  .data : AT(ADDR(.data) - KERNEL_BASE) {
    PROVIDE(__data_start = .);
    *(.data .data.*)
    *(.got .got.*)
    . = ALIGN(0x100000);
    *(.runtime)
    . = ALIGN(0x1000);
    PROVIDE(__data_end = .);
  }

  .bss : AT(ADDR(.bss) - KERNEL_BASE) {
    PROVIDE(__edata_start = .);
    *(.bss .bss.*)
    . = ALIGN(0x1000);
    PROVIDE(__edata_end = .);
  }

  /* DWARF debug sections.
     Symbols in the DWARF debugging sections are relative to the beginning
     of the section so we begin them at 0.  */
  .debug          0 : { *(.debug) }
  .line           0 : { *(.line) }
  .debug_srcinfo  0 : { *(.debug_srcinfo) }
  .debug_sfnames  0 : { *(.debug_sfnames) }
  .debug_aranges  0 : { *(.debug_aranges) }
  .debug_pubnames 0 : { *(.debug_pubnames) }
  .debug_info     0 : { *(.debug_info .gnu.linkonce.wi.*) }
  .debug_abbrev   0 : { *(.debug_abbrev) }
  .debug_line     0 : { *(.debug_line) }
  .debug_frame    0 : { *(.debug_frame) }
  .debug_str      0 : { *(.debug_str) }
  .debug_loc      0 : { *(.debug_loc) }
  .debug_macinfo  0 : { *(.debug_macinfo) }
  .debug_weaknames 0 : { *(.debug_weaknames) }
  .debug_funcnames 0 : { *(.debug_funcnames) }
  .debug_typenames 0 : { *(.debug_typenames) }
  .debug_varnames  0 : { *(.debug_varnames) }
  .debug_pubtypes 0 : { *(.debug_pubtypes) }
  .debug_ranges   0 : { *(.debug_ranges) }
  .debug_macro    0 : { *(.debug_macro) }
  .gnu.attributes 0 : { KEEP (*(.gnu.attributes)) }
  .stack_sizes    0 : { *(.stack_sizes) }

  /DISCARD/ : { *(.comment) *(.gnu*) *(.note*) }
}
