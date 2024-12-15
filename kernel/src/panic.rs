#![cfg_attr(miri, allow(unused_imports))]
use crate::{io::uart::QEMU_UART, println, test::qemu_exit::wait_for_the_end};
use core::{panic::PanicInfo, sync::atomic::AtomicU8};

#[cfg(test)]
use crate::test::qemu_exit::exit_failure;

static PANIC_COUNTER: AtomicU8 = AtomicU8::new(0);

#[cfg(not(miri))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crate::cpu::Cpu;

    unsafe {
        crate::Cpu::disable_global_interrupts();
    }

    // SAFTEY: The worst what happen is scrambled output
    // Disable the stdout mutex in case it was locked before
    // This is not safe but useful in case we panic while we are
    // output some data
    unsafe {
        QEMU_UART.disarm();
    }

    let cpu = unsafe { Cpu::current_nevertheless() };

    println!("");
    println!("KERNEL Panic Occured on cpu {}!", Cpu::cpu_id());
    println!("Message: {}", info.message());
    if let Some(location) = info.location() {
        println!("Location: {}", location);
    }
    println!("Kernel Page Tables {}", cpu.kernel_page_table());
    abort_if_double_panic();
    crate::debugging::backtrace::print();
    crate::debugging::dump_current_state();

    println!("Time to attach gdb ;) use 'just attach'");

    #[cfg(test)]
    exit_failure(1);

    #[cfg(not(test))]
    wait_for_the_end();
}

fn abort_if_double_panic() {
    let current = PANIC_COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst);

    if current >= 1 {
        println!("Panic in panic! ABORTING!");
        println!("Time to attach gdb ;) use 'just attach'");

        #[cfg(test)]
        exit_failure(1);

        #[cfg(not(test))]
        wait_for_the_end();
    }
}
