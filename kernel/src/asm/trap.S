.attribute arch, "rv64gc"
# Copied from https://osblog.stephenmarz.com/ch4.html
.altmacro
.set NUM_GP_REGS, 32  # Number of registers per context
.set NUM_FP_REGS, 32
.set REG_SIZE, 8   # Register size (in bytes)

# Use macros for saving and restoring multiple registers
.macro save_gp i, basereg=t6
	sd	x\i, ((\i)*REG_SIZE)(\basereg)
.endm
.macro load_gp i, basereg=t6
	ld	x\i, ((\i)*REG_SIZE)(\basereg)
.endm
.macro save_fp i, basereg=t6
	fsd	f\i, ((NUM_GP_REGS+(\i))*REG_SIZE)(\basereg)
.endm
.macro load_fp i, basereg=t6
	fld	f\i, ((NUM_GP_REGS+(\i))*REG_SIZE)(\basereg)
.endm

.macro save_regs
	# Save all registers to the kernel trap frame
	csrrw t6, sscratch, t6

	# Add trap frame offset
	addi t6, t6, {TRAP_FRAME_OFFSET}

	# Save all registers
	# In order to make things easier we also save x0 which is always zero
	.set 	i, 0
	.rept	31
		save_gp	%i
		save_fp %i
		.set	i, i+1
	.endr
	
	save_fp 31

	# Save last register
	mv t5, t6
	csrr t6, sscratch
	save_gp 31, t5

	# Remove trap frame offset
	addi t5, t5, -{TRAP_FRAME_OFFSET}

	# Restore trap frame pointer
	csrw sscratch, t5
.endm

.macro restore_regs
	# Restore all registers
	csrr t6, sscratch

	# Add trap frame offset
	addi t6, t6, {TRAP_FRAME_OFFSET}

	.set i,0
	.rept 32
		load_fp %i # We need to restore the floating point register first. Otherwise the last load_gp overwrites the address of the trap frame
		load_gp %i
		.set i, i+1
	.endr
.endm

.macro handler func
.section .text
.global asm_\func
.align 4
asm_\func:
	
	save_regs

	# Load kernel page tables such that we get the correct stack
	csrr t0, sscratch
	ld t1, {KERNEL_PAGE_TABLES_SATP_OFFSET}(t0)
	csrw satp, t1
	sfence.vma zero, zero

	# Set kernel stack pointer to 0xffffffffffffffff
	li sp, 0
	addi sp, sp, -64

	call \func

	# Restore the process page table
	call get_process_satp_value
	csrw satp, a0
	sfence.vma
	
	restore_regs

	sret

.endm

handler handle_exception
handler handle_unimplemented
handler handle_timer_interrupt
handler handle_external_interrupt

.section .text
.global supervisor_trap_table
.align 4
supervisor_trap_table:
	j asm_handle_exception
	j asm_handle_unimplemented        # cause: 1
	j asm_handle_unimplemented        # cause: 2
	j asm_handle_unimplemented        # cause: 3
	j asm_handle_unimplemented        # cause: 4
	j asm_handle_timer_interrupt      # cause: 5
	j asm_handle_unimplemented        # cause: 6
	j asm_handle_unimplemented        # cause: 7
	j asm_handle_unimplemented        # cause: 8
	j asm_handle_external_interrupt   # cause: 9
	j asm_handle_unimplemented        # cause: 10
	j asm_handle_unimplemented        # cause: 11
	j asm_handle_unimplemented        # cause: 12
	j asm_handle_unimplemented        # cause: 13
	j asm_handle_unimplemented        # cause: 14
	j asm_handle_unimplemented        # cause: 15

