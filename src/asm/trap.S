.section .text
.global machine_trap_vector
.align 4
machine_trap_vector:
	csrr a0, mcause
	csrr a1, mtval
	jal machine_mode_trap
	wfi
	j machine_trap_vector

.global supervisor_trap_vector
.align 4
supervisor_trap_vector:
	csrr a0, mcause
	csrr a1, mtval
	jal supervisor_mode_trap
	wfi
	j supervisor_trap_vector