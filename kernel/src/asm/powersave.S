.section .text
.global powersave
.align 4
powersave:
        # Reduce timer interrupts when in powersave mode
        li a0, 50
        call set_timer

        wfi
        j powersave
