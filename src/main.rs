#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(panic_info_message)]

mod asm;
mod heap;
mod mmio;
mod println;
mod uart;

use core::panic::PanicInfo;

extern crate alloc;

use alloc::vec::Vec;

#[no_mangle]
extern "C" fn kernel_init() {
    uart::QEMU_UART.init();
    println!("Hello World from YaROS!");
    let mut x: Vec<u8> = Vec::new();
    x.push(1);
    x.push(1);
    x.push(1);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic Occured!");
    if let Some(message) = info.message() {
        println!("Message: {}", message);
    }
    if let Some(location) = info.location() {
        println!("Location: {}", location);
    }
    loop {}
}
