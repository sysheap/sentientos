use common::syscalls::sys_exit;

use crate::{args, println};

#[unsafe(no_mangle)]
#[linkage = "weak"]
pub extern "C" fn _start(args: *const u8) -> ! {
    args::init(args);
    main();
    sys_exit(0);
    panic!();
}

#[unsafe(no_mangle)]
#[linkage = "weak"]
pub extern "C" fn main() {
    println!("Default Main");
}
