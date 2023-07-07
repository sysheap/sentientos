#![no_std]

use core::panic::PanicInfo;

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
