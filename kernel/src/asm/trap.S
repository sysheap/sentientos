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

.section .text
.global machine_trap_vector
.align 4
machine_trap_vector:
	# Save all registers to the kernel trap frame
	csrrw t6, mscratch, t6

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
	csrr t6, mscratch
	save_gp 31, t5

	# Restore trap frame pointer
	csrw mscratch, t5

	csrr a0, mcause
	csrr a1, mtval
	csrr a2, mepc
	csrr a3, mscratch
	call machine_mode_trap

	# Restore all registers
	csrr t6, mscratch

	.set i,0
	.rept 32
		load_fp %i # We need to restore the floating point register first. Otherwise the last load_gp overwrites the address of the trap frame
		load_gp %i
		.set i, i+1
	.endr

	mret

.global supervisor_trap_vector
.align 4
supervisor_trap_vector:
	csrr a0, mcause
	csrr a1, mtval
	call supervisor_mode_trap
	mret
