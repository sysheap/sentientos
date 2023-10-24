.option norvc
.section .text
.global _start

.equ MSTATUS_SIE, (1 << 1)
.equ MSTATUS_MIE, (1 << 3)
.equ MSTATUS_SPIE, (1 << 5)
.equ MSTATUS_MPIE, (1 << 7)
.equ MSTATUS_SPP, (1 << 8)
.equ MSTATUS_MPP_LOW, (1 << 11)
.equ MSTATUS_MPP_HIGH, (1 << 12)
.equ MSTATUS_MPP, (MSTATUS_MPP_LOW | MSTATUS_MPP_HIGH)
.equ MIP_MTIP, (1 << 7)
.equ MIP_STIP, (1 << 5)

.equ TIMER_CURRENT, 0x0200bff8
.equ TIMER_COMPARE, 0x02004000
.equ TIMER_TICKS, 10000

.equ MCAUSE_TIMER_INTERRUPT, 0x8000000000000007

.equ BIT_64_ALL_ONES, 0xffffffffffffffff

_start:

    # Disable interrupts
    csrw mie, zero

    # load machine mode trap
    la t0, mtrap
    csrw mtvec, t0

    # load supervisor mode trap
    la t0, strap
    csrw stvec, t0

    # set MPP to 01 (supervisor mode)
    li t0, MSTATUS_MPP
    csrc mstatus, t0

    li t0, MSTATUS_MPP_LOW
    csrs mstatus, t0

    # Set previous interrupt-enable bit to 1
    li t0, MSTATUS_MPIE
    or t0, t0, MSTATUS_SPIE
    csrs mstatus, t0

    # Enable all interrupts
    li t0, 0b0101010101010
    csrw mie, t0

    # Enable supervisor interrupts
    li t0, MSTATUS_SIE
    csrs mstatus, t0

	# We need at least one pmp entry. Therefore we create on which spans the whole address space.
	li t0, -1
	csrw pmpaddr0, t0
	csrw pmpcfg0, 0xF

    # set supervisor function
    la t0, supervisor_mode
    csrw mepc, t0

    # delegate all interrupts and exceptions to supervisor mode
    li t0, -1
    csrw medeleg, t0
    csrw mideleg, t0

    # Jump to supervisor mode
    mret

    j _start

supervisor_mode:
    # Set timer
    li t0, TIMER_CURRENT
    ld t1, 0(t0) # Current timer value
    li t2, 100 * TIMER_TICKS
    add t1, t1, t2
    li t0, TIMER_COMPARE
    sd t1, 0(t0) # Set new timer value

    wfi
    j supervisor_mode

.align 16
mtrap:
    # Forward interrupt to supervisor mode if timer interrupt
    li t0, MCAUSE_TIMER_INTERRUPT
    csrr t1, mcause
    bne t0, t1, ret

    # Reset timer interrupt
    li t0, BIT_64_ALL_ONES
    li t1, TIMER_COMPARE
    sd t0, 0(t1)

    li t0, MIP_STIP
    csrs mip, t0

ret:
    mret

.align 16
strap:
    # Set timer
    li t0, TIMER_CURRENT
    ld t1, 0(t0) # Current timer value
    li t2, 100 * TIMER_TICKS
    add t1, t1, t2
    li t0, TIMER_COMPARE
    sd t1, 0(t0) # Set new timer value
    
    sret


# .option norvc
# .section .data
# .section .text.init
# .global _start

# _start:
#     li      t0, 0x1800
#     csrc    mstatus, t0
#     li      t0, 0x802
#     csrs    mstatus, t0
#     li      t0, 0xa0 
#     csrs    mie, t0
#     li      t0, 0x20
#     csrs    mideleg, t0
#     la      t0, mtrap
#     csrw    mtvec, t0
#     li      t0, 500000
#     li      t1, 0x2004000
#     sd      t0, 0(t1)
#     la      t1, supervisor
#     csrw    mepc, t1

# 	# We need at least one pmp entry. Therefore we create on which spans the whole address space.
# 	li t0, -1
# 	csrw pmpaddr0, t0
# 	csrw pmpcfg0, 0xF

#     mret

# mtrap:
#     li      t0, 0x20
#     csrs    mip, t0
#     mret

# supervisor:
#     li      t0, 20
#     la      t0, strap
#     csrw    stvec, t0
#     j       spin

# strap:
#     j       spin

# spin:
#     j       spin