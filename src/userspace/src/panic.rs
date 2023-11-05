use core::panic::PanicInfo;

use crate::println;

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("");
    println!("USERSPACE Panic Occured!");
    if let Some(message) = info.message() {
        println!("Message: {}", message);
    }
    if let Some(location) = info.location() {
        println!("Location: {}", location);
    }

    loop {}
}
