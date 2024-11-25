/*
 Linker script for outputting to RISC-V QEMU "virt" machine.
 Based on Stephen Marz
 6 October 2019
*/

OUTPUT_ARCH( "riscv" )

ENTRY( _start )

MEMORY_START = 0x80200000;


SECTIONS
{
  . = MEMORY_START;

  .text : {
    PROVIDE(__text_start = .);
    PROVIDE(__executable_start = .);
    *(.text.init) *(.text .text.*)
    PROVIDE(__etext = .);
	. = ALIGN(4096);
    PROVIDE(__text_end = .);
  }

  .rodata : {
	. = ALIGN(4096);
    PROVIDE(__rodata_start = .);
    *(.rodata .rodata.* .srodata .srodata.*)
	. = ALIGN(4096);
    PROVIDE(__rodata_end = .);
  }

  .eh_frame : {
	. = ALIGN(4096);
    /* PROVIDE(__eh_frame_section_start = .); */
    PROVIDE(__eh_frame = .);
    PROVIDE(__eh_frame_start = .);
    KEEP (*(.eh_frame)) *(.eh_frame.*)
    /* PROVIDE(__eh_frame_end = .); */
	. = ALIGN(4096);
    PROVIDE(__eh_frame_end = .);
  }

  .data : {
    . = ALIGN(4096);
    PROVIDE(__data_start = .);
   PROVIDE(__global_pointer$ = . + 0x800);
    *(.sdata .sdata.*) *(.data .data.*)
	. = ALIGN(4096);
    PROVIDE(__data_end = .);
  }

  .bss (NOLOAD) : {
	. = ALIGN(4096);
    PROVIDE(__bss_start = .);
    *(.sbss .sbss.*) *(.bss .bss.*)
	. = ALIGN(4096);
    PROVIDE(__bss_end = .);
  }

  .kernel_stack (NOLOAD) : {
    . = ALIGN(4096);
    PROVIDE(__kernel_stack_start = .);
    . = . + 512K;
    . = ALIGN(4096);
    PROVIDE(__kernel_stack_end = .);
  }

  .heap (NOLOAD) : {
    . = ALIGN(4096);
    PROVIDE(__heap_start = .);
    # The end of the heap will be calcualted at runtime
  }

  /* PROVIDE(_eh_frame_size = _eh_frame_end - _eh_frame_section_start); */
}
