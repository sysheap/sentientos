OUTPUT_ARCH( "riscv" )
ENTRY( _start )

SECTIONS
{
	. = 0x1000; /* Keeps the zero page empty */
	
	.text ALIGN(4K) : {
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
