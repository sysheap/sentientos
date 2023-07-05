use core::panic::PanicInfo;

use crate::{println, test};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic Occured!");
    if let Some(message) = info.message() {
        println!("Message: {}", message);
    }
    if let Some(location) = info.location() {
        println!("Location: {}", location);
    }
    test::qemu_exit::exit_failure(1);
    loop {}
}
