.section .text
.global powersave
.align 4
powersave:
        # Disable timer interrupt
        li t0, (1 << 5)
        csrc sie, t0

        # Set next timer interrupt to -2
        li a7, {EID}
        li a6, {FID_SET_TIMER}

        # arg
        li a0, 0
        # Somehow u64::MAX triggers timer interrupt indefinitely
        # u64::MAX - 1 works as intended.
        addi a0, a0, -2

        ecall

        # a0 should be 0 to indicate success
        bne a0, x0, powersave_error

        wfi
        j powersave

powersave_error:
        call asm_panic
powersave_error_sleep:
        wfi
        j powersave_error_sleep
