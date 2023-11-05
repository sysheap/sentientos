extern "C" {
    fn main();
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        main();
    }
    common::syscalls::userspace::EXIT(0);
    loop {}
}
