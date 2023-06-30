#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(panic_info_message)]
#![feature(pointer_byte_offsets)]
#![feature(strict_provenance)]
#![feature(nonzero_ops)]
#![feature(core_intrinsics)]

mod asm;
mod heap;
mod mmio;
mod page_allocator;
mod page_tables;
mod println;
mod uart;
mod util;

use core::panic::PanicInfo;

// extern crate alloc;

extern "C" {
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}

#[no_mangle]
extern "C" fn kernel_init() {
    uart::QEMU_UART.init();
    println!("Hello World from YaROS!");
    unsafe {
        page_allocator::init(HEAP_START, HEAP_SIZE);
    }

    page_tables::setup_kernel_identity_mapping();
    println!("Boot completed!");
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
