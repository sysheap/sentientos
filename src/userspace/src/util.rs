use core::arch::asm;

pub fn wait() {
    for _ in 0..100000000 {
        unsafe {
            asm!("nop");
        }
    }
}
