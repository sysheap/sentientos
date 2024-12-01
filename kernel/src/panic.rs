#![cfg_attr(miri, allow(unused_imports))]
use crate::{io::uart::QEMU_UART, println, test::qemu_exit};
use core::{panic::PanicInfo, sync::atomic::AtomicU8};

static PANIC_COUNTER: AtomicU8 = AtomicU8::new(0);

#[cfg(not(miri))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crate::memory::page_tables::KERNEL_PAGE_TABLES;

    crate::cpu::disable_gloabl_interrupts();

    // SAFTEY: The worst what happen is scrambled output
    // Disable the stdout mutex in case it was locked before
    // This is not safe but useful in case we panic while we are
    // output some data
    unsafe {
        QEMU_UART.disarm();
    }

    println!("");
    println!("KERNEL Panic Occured!");
    println!("Message: {}", info.message());
    if let Some(location) = info.location() {
        println!("Location: {}", location);
    }
    println!("Kernel Page Tables {}", &*KERNEL_PAGE_TABLES);
    abort_if_double_panic();
    crate::debugging::backtrace::print();
    crate::debugging::dump_current_state();

    qemu_exit::exit_failure(1);
}

fn abort_if_double_panic() {
    let current = PANIC_COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst);

    if current >= 1 {
        println!("Panic in panic! ABORTING!");
        qemu_exit::exit_failure(1);
    }
}
