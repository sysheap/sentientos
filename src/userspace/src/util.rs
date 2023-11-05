use core::arch::asm;

pub fn wait() {
    for _ in 0..10000 {
        unsafe {
            asm!("nop");
        }
    }
}
