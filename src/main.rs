#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unused_variables)]

mod asm;
mod mmio;
mod println;
mod uart;

use core::panic::PanicInfo;

#[no_mangle]
extern "C" fn kernel_init() {
    uart::QEMU_UART.init();
    println!("Hello World from YaROS!");
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
