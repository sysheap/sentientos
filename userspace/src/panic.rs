use core::panic::PanicInfo;

use common::syscalls::sys_exit;

use crate::println;

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("");
    println!("USERSPACE Panic Occured!");
    println!("Message: {}", info.message());
    if let Some(location) = info.location() {
        println!("Location: {}", location);
    }

    sys_exit(-1);
    loop {}
}
