use common::syscalls::sys_exit;

use crate::args;

unsafe extern "C" {
    fn main();
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(args: *const u8) -> ! {
    args::init(args);
    unsafe {
        main();
    }
    sys_exit(0);
    #[allow(clippy::empty_loop)]
    loop {}
}
