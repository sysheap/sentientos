.section .text
.global machine_trap_vector
.align 4
machine_trap_vector:
	csrr a0, mcause
	csrr a1, mtval
	call machine_mode_trap
	mret

.global supervisor_trap_vector
.align 4
supervisor_trap_vector:
	csrr a0, mcause
	csrr a1, mtval
	call supervisor_mode_trap
	mret