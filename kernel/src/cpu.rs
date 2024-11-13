use core::arch::asm;

use common::syscalls::trap_frame::TrapFrame;

pub fn write_sscratch_register(value: *const TrapFrame) {
    unsafe {
        asm!("csrw sscratch, {}", in(reg) value);
    }
}

pub fn write_sepc(value: usize) {
    unsafe {
        asm!("csrw sepc, {}", in(reg) value);
    }
}

pub fn read_sepc() -> usize {
    let sepc: usize;
    unsafe {
        asm!("csrr {}, sepc", out(reg) sepc);
    }
    sepc
}

pub unsafe fn write_satp_and_fence(satp_val: usize) {
    unsafe {
        asm!("csrw satp, {satp_val}", satp_val = in(reg) satp_val);
        asm!("sfence.vma");
    }
}

pub fn read_satp() -> usize {
    if cfg!(miri) {
        return 0;
    }

    let satp: usize;
    unsafe {
        asm!("csrr {}, satp", out(reg) satp);
    }
    satp
}

pub fn memory_fence() {
    unsafe {
        asm!("fence");
    }
}

pub fn disable_gloabl_interrupts() {
    unsafe {
        asm!(
            "li t0, 0b10
            csrc sstatus, t0"
        );
    }
}

pub fn wait_for_interrupt() {
    unsafe {
        asm!("wfi");
    }
}
