#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(panic_info_message)]
#![feature(pointer_byte_offsets)]
#![feature(strict_provenance)]
#![feature(nonzero_ops)]
#![feature(core_intrinsics)]
#![feature(custom_test_frameworks)]
#![test_runner(test::test_runner)]
#![reexport_test_harness_main = "test_main"]

use crate::{
    interrupts::plic,
    io::uart,
    memory::{heap, page_allocator, page_tables},
};

mod asm;
mod interrupts;
mod io;
mod klibc;
mod memory;
mod panic;
mod test;

extern crate alloc;

extern "C" {
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}

#[no_mangle]
extern "C" fn kernel_init() {
    uart::QEMU_UART.init();
    println!("Hello World from YaROS!\n");

    #[cfg(test)]
    test_main();

    unsafe {
        println!("Initializing page allocator");
        page_allocator::init(HEAP_START, HEAP_SIZE);
        heap::init();
    }

    page_tables::setup_kernel_identity_mapping();
    interrupts::set_mscratch_to_kernel_trap_frame();

    println!("kernel_init() completed!");
}

#[no_mangle]
extern "C" fn kernel_main() {
    println!("kernel_main()");

    plic::init_uart_interrupt();
}
