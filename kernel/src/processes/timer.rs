use core::arch::asm;

use crate::{klibc::MMIO, sbi};

pub const CLINT_BASE: usize = 0x2000000;
pub const CLINT_SIZE: usize = 0x10000;

const TIMER_COMPARE_REGISTER: MMIO<u64> = MMIO::new(0x0200_4000);
const TIMER_CURRENT_REGISTER: MMIO<u64> = MMIO::new(0x0200_bff8);

const CLOCKS_PER_MSEC: u64 = 10_000;

pub fn set_timer(milliseconds: u64) {
    let current = get_current_clocks();
    let next = current + (CLOCKS_PER_MSEC * milliseconds);
    sbi::extensions::timer_extension::sbi_set_timer(next).assert_success();
}

pub fn disable_timer() {
    sbi::extensions::timer_extension::sbi_set_timer(u64::MAX).assert_success();
}

fn get_current_clocks() -> u64 {
    let current: u64;
    unsafe {
        asm!("rdtime {current}", current = out(reg)current);
    };
    current
}
