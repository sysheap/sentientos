use common::syscalls::sys_exit;

unsafe extern "C" {
    fn main();
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe {
        main();
    }
    sys_exit(0);
    #[allow(clippy::empty_loop)]
    loop {}
}
