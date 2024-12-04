# Disable generation of compressed instructions.
.option norvc

# Define a .text.init section. The .text.init is put at the
# starting address so that the entry _start is put at the RISC-V
# address 0x8000_0000.
.section .text.init

# Execution starts here.
# a0 is hart id
# a1 is pointer to device tree
# Preserve them during startup code
.global _start
_start:

	# Disable linker instruction relaxation for the `la` instruction below.
	# This disallows the assembler from assuming that `gp` is already initialized.
	# This causes the value stored in `gp` to be calculated from `pc`.
	# The job of the global pointer is to give the linker the ability to address
	# memory relative to GP instead of as an absolute address.
.option push
.option norelax
	la		gp, __global_pointer$
.option pop

	# Disable interrupts.
	csrw sie, zero

	# Clear bss segment
	la t0, __start_bss
	la t1, __stop_bss
	bgeu t0, t1, bss_cleared

bss_loop:
	sd zero, (t0)
	addi t0, t0, 8
	bltu t0, t1, bss_loop

bss_cleared:

	la sp, __stop_kernel_stack

	# Load trap vector machine mode
	la t0, supervisor_trap_vector
	csrw stvec, t0

	call kernel_init

	# Enable all interrupts
	li t0, -1
	csrw sie, t0

	# Enable global interrupts
	li t0, 0b10
	csrs sstatus, t0

wfi_loop:
	wfi
	j wfi_loop
