#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(panic_info_message)]
#![feature(pointer_byte_offsets)]

mod asm;
mod heap;
mod mmio;
mod println;
mod uart;

use core::panic::PanicInfo;

extern crate alloc;

use alloc::{string::String, vec::Vec};

#[no_mangle]
extern "C" fn kernel_init() {
    uart::QEMU_UART.init();
    println!("Hello World from YaROS!");

    heap::init();

    {
        let mut x: Vec<u8> = Vec::new();
        x.push(1);
    }
    {
        let mut x: Vec<u8> = Vec::new();
        x.push(1);

        let mut y: Vec<u8> = Vec::new();
        y.push(1);
        y.push(1);
        y.push(1);
        y.push(1);
    }
    let mut x = String::from("Hello!");
    for _ in 0..1000 {
        x += "hello";
    }
    println!("{}", x);
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
    heap::dump();
    loop {}
}
