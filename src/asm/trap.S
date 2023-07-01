.section .text
.global asm_trap_vector
asm_trap_vector:
asm_trap_vector_loop:
	wfi
	j asm_trap_vector_loop