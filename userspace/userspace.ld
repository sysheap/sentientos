OUTPUT_ARCH( "riscv" )
ENTRY( _start )

SECTIONS
{
	. = 0x1000;
	
	.text : {
		*(.text .text.*)
	}
	
	.rodata ALIGN(4K) : {
		*(.srodata .srodata.*)
		*(.rodata .rodata.*)
	}
	
	.data ALIGN(4K) : {
		*(.sdata .sdata.*)
		*(.data .data.*)
	}
	
	.bss ALIGN(4K) : {
		*(.sbss .sbss.*)
		*(.bss .bss.*)
	}
}
