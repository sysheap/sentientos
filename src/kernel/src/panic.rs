#![cfg_attr(miri, allow(unused_imports))]
use crate::{println, test::qemu_exit};
use core::panic::PanicInfo;

#[cfg(not(miri))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::cpu::disable_gloabl_interrupts();
    println!("");
    crate::debug::dump_current_state();
    println!("KERNEL Panic Occured!");
    if let Some(message) = info.message() {
        println!("Message: {}", message);
    }
    if let Some(location) = info.location() {
        println!("Location: {}", location);
    }

    qemu_exit::exit_failure(1);
}
