.section .text
.global machine_trap_vector
.align 4
machine_trap_vector:
	wfi
	j machine_trap_vector

.global supervisor_trap_vector
.align 4
supervisor_trap_vector:
	wfi
	j supervisor_trap_vector