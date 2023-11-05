#![no_std]
#![allow(dead_code)]
#![allow(unused_variables)]

use core::{arch::asm, panic::PanicInfo};

extern "C" {
    fn main();
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        main();
    }
    #[allow(clippy::empty_loop)]
    loop {}
}

#[panic_handler]
pub fn panic(_: &PanicInfo) -> ! {
    loop {}
}

pub fn wait() {
    for _ in 0..100000000 {
        unsafe {
            asm!("nop");
        }
    }
}
