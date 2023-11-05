extern "C" {
    fn main();
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        main();
    }
    common::syscalls::userspace::EXIT(0);
    #[allow(clippy::empty_loop)]
    loop {}
}
