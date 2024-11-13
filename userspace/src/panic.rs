use core::panic::PanicInfo;

use crate::println;

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("");
    println!("USERSPACE Panic Occured!");
    println!("Message: {}", info.message());
    if let Some(location) = info.location() {
        println!("Location: {}", location);
    }

    loop {}
}
