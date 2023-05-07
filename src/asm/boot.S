# Disable generation of compressed instructions.
.option norvc

# Define a .text.init section. The .text.init is put at the
# starting address so that the entry _start is put at the RISC-V
# address 0x8000_0000.
.section .text.init

# Execution starts here.
.global _start
_start:

	# Disable linker instruction relaxation for the `la` instruction below.
	# This disallows the assembler from assuming that `gp` is already initialized.
	# This causes the value stored in `gp` to be calculated from `pc`.
	# The job of the global pointer is to give the linker the ability to address
	# memory relative to GP instead of as an absolute address.
.option push
.option norelax
	la		gp, _global_pointer
.option pop
	
	# Clear bss segment
	la a0, _bss_start
	la a1, _bss_end
	bgeu a0, a1, bss_cleared

bss_loop:
	sd zero, (a0)
	addi a0, a0, 8
	bltu a0, a1, bss_loop

bss_cleared:
	la sp, _stack_end
	li t0, (0b11 << 11) | (1 << 7) | (1 << 3) # MIE, MPIE, MPP
	csrw mstatus, t0

	la t0, kernel_init
	csrw mepc, t0

	la t0, asm_trap_vector
	csrw mtvec, t0

	li t0, (1 << 3) | (1 << 7) | (1 << 11)
	csrw mie, t0

	la ra, loop
	mret

loop:
	wfi
	j loop