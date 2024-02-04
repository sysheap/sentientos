use common::syscalls::sys_exit;

extern "C" {
    fn main();
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        main();
    }
    sys_exit(0);
    #[allow(clippy::empty_loop)]
    loop {}
}
