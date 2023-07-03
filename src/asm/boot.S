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

	# Disable interrupts.
	csrw mie, zero

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

	jal kernel_init

	# Jump to supervisor mode
	la t0, kernel_main
	csrw mepc, t0

	# Load trap vector machine mode
	la t0, machine_trap_vector
	csrw mtvec, t0

	# Load trap vector supervisor mode
	la t0, supervisor_trap_vector
	csrw stvec, t0

	# Set MPP to 01 (supervisor mode)
	li t0, 0x1800
	csrc mstatus, t0

	li t0, 0x800
	csrs mstatus, t0

	# We need at least one pmp entry. Therefore we create on which spans the whole address space.
	li t0, -1
	csrw pmpaddr0, t0
	csrw pmpcfg0, 0xF

	la ra, loop

activate_supervisor_mode:
	# Jump to supervisor mode
	mret

loop:
	wfi
	j loop