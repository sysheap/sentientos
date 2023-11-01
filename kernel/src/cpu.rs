use core::arch::asm;

use crate::interrupts::trap::TrapFrame;

pub fn write_sscratch_register(value: *const TrapFrame) {
    unsafe {
        asm!("csrw sscratch, {}", in(reg) value);
    }
}

pub fn write_sepc_register(value: usize) {
    unsafe {
        asm!("csrw sepc, {}", in(reg) value);
    }
}

pub fn get_sepc() -> usize {
    let sepc: usize;
    unsafe {
        asm!("csrr {}, sepc", out(reg) sepc);
    }
    sepc
}
