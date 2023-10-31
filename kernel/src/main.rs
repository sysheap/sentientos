#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(panic_info_message)]
#![feature(strict_provenance)]
#![feature(nonzero_ops)]
#![feature(core_intrinsics)]
#![feature(custom_test_frameworks)]
#![test_runner(test::test_runner)]
#![reexport_test_harness_main = "test_main"]

use alloc::rc::Rc;

use crate::{
    interrupts::plic,
    io::uart::QEMU_UART,
    memory::{
        heap, page_allocator,
        page_tables::{self, RootPageTableHolder},
    },
    processes::{scheduler, timer},
};

mod asm;
mod interrupts;
mod io;
mod klibc;
mod memory;
mod panic;
mod processes;
mod sbi;
mod test;

extern crate alloc;

extern "C" {
    static HEAP_START: usize;
    static HEAP_SIZE: usize;
}

#[no_mangle]
extern "C" fn kernel_init() {
    QEMU_UART.lock().init();

    println!("Hello World from YaROS!\n");

    let version = sbi::extensions::base_extension::sbi_get_spec_version();
    println!("SBI version {}.{}", version.major, version.minor);
    assert!(
        (version.major == 0 && version.minor >= 2) || version.major > 0,
        "Supported SBI Versions >= 0.2"
    );

    unsafe {
        println!("Initializing page allocator");
        page_allocator::init(HEAP_START, HEAP_SIZE);
        heap::init();
    }

    #[cfg(test)]
    test_main();

    page_tables::activate_page_table(Rc::new(RootPageTableHolder::new_with_kernel_mapping()));
    interrupts::set_sscratch_to_kernel_trap_frame();

    plic::init_uart_interrupt();

    scheduler::SCHEDULER.lock().initialize();
    timer::set_timer(1000);
}
