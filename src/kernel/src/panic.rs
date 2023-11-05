use core::panic::PanicInfo;

use crate::println;

#[cfg(test)]
use crate::test;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("");
    println!("KERNEL Panic Occured!");
    if let Some(message) = info.message() {
        println!("Message: {}", message);
    }
    if let Some(location) = info.location() {
        println!("Location: {}", location);
    }

    #[cfg(test)]
    test::qemu_exit::exit_failure(1);

    loop {}
}
