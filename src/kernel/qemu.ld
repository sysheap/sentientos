/*
 Linker script for outputting to RISC-V QEMU "virt" machine.
 Based on Stephen Marz
 6 October 2019
*/

OUTPUT_ARCH( "riscv" )

ENTRY( _start )

MEMORY
{
  ram  (wxa) : ORIGIN = 0x80200000, LENGTH = 126M
}

PHDRS
{
  text PT_LOAD;
  data PT_LOAD;
  bss PT_LOAD;
}

SECTIONS
{
  .text : {

    PROVIDE(_text_start = .);
    *(.text.init) *(.text .text.*)
	. = ALIGN(4096);
    PROVIDE(_text_end = .);
  } >ram AT>ram :text

  .rodata : {
	. = ALIGN(4096);
    PROVIDE(_rodata_start = .);
    *(.rodata .rodata.*)
	. = ALIGN(4096);
    PROVIDE(_rodata_end = .);
  } >ram AT>ram :text

  .data : {
    . = ALIGN(4096);
    PROVIDE(_data_start = .);
   PROVIDE(__global_pointer$ = . + 0x800);
    *(.sdata .sdata.*) *(.data .data.*)
	. = ALIGN(4096);
    PROVIDE(_data_end = .);
  } >ram AT>ram :data

  .bss : {
	. = ALIGN(4096);
    PROVIDE(_bss_start = .);
    *(.sbss .sbss.*) *(.bss .bss.*)
	. = ALIGN(4096);
    PROVIDE(_bss_end = .);
  } >ram AT>ram :bss

  PROVIDE(_memory_start = ORIGIN(ram));
  PROVIDE(_stack_start = _bss_end);
  PROVIDE(_stack_end = _stack_start + 0x80000);
  PROVIDE(_memory_end = ORIGIN(ram) + LENGTH(ram));

  PROVIDE(_heap_start = _stack_end);
  PROVIDE(_heap_size = _memory_end - _heap_start);
}
