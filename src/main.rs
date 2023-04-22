#![no_std]
#![no_main]

mod asm;

use core::panic::PanicInfo;

#[no_mangle]
extern "C" fn kernel_init() {
    loop {}
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
