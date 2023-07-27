use crate::klibc::MMIO;

pub const CLINT_BASE: usize = 0x2000000;
pub const CLINT_SIZE: usize = 0x10000;

const TIMER_COMPARE_REGISTER: MMIO<u64> = MMIO::new(0x0200_4000);
const TIMER_CURRENT_REGISTER: MMIO<u64> = MMIO::new(0x0200_bff8);

pub fn set_timer(milliseconds: u64) {
    unsafe {
        let current = TIMER_CURRENT_REGISTER.read();
        let next = current + (10000 * milliseconds);
        TIMER_COMPARE_REGISTER.write(next);
    }
}
