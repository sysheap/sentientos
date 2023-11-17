use core::arch::asm;

use crate::sbi;

pub const CLINT_BASE: usize = 0x2000000;
pub const CLINT_SIZE: usize = 0x10000;

const CLOCKS_PER_MSEC: u64 = 10_000;

pub fn set_timer(milliseconds: u64) {
    let current = get_current_clocks();
    let next = current + (CLOCKS_PER_MSEC * milliseconds);
    sbi::extensions::timer_extension::sbi_set_timer(next).assert_success();
}

fn get_current_clocks() -> u64 {
    let current: u64;
    unsafe {
        asm!("rdtime {current}", current = out(reg)current);
    };
    current
}
