#![no_std]
#![no_main]

mod asm;

use core::panic::PanicInfo;

#[no_mangle]
extern "C" fn kernel_init() {
    panic!();
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
