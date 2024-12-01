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

  text : ALIGN(4K) {
    PROVIDE(__executable_start = .);
    *(.text.init) *(.text .text.*)
    PROVIDE(__etext = .);
  }

  rodata : ALIGN(4K) {
    *(.rodata .rodata.* .srodata .srodata.*)
  }

  eh_frame : ALIGN(4K) {
    PROVIDE(__eh_frame = .);
    KEEP (*(.eh_frame)) *(.eh_frame.*)
  }

  data : ALIGN(4K) {
    PROVIDE(__global_pointer$ = . + 0x800);
    *(.sdata .sdata.*) *(.data .data.*)
  }

  bss (NOLOAD) : ALIGN(4K) {
    *(.sbss .sbss.*) *(.bss .bss.*)
  }

  # This page will be an unmapped area to prevent stack overflows
  # I know - it is not the most efficient way because we waste some actual
  # memory here. It would be better to only have a guard page in virtual memory.
  # However, this needs more redesigning about adress space layout
  # so lets leave this here as quick fix
  # The guard page ist not aligned such that it starts directly after bss
  # and continues until the kernel_stack
  stack_overflow_guard (NOLOAD) : ALIGN(4K) {
    . = . + 1;
  }

  kernel_stack (NOLOAD) : ALIGN(4K) {
    . = . + 512K;
  }

  heap (NOLOAD) : ALIGN(4K) {
    # The end of the heap will be calcualted at runtime
  }
}
