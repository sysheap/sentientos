#![no_std]

use core::{arch::asm, panic::PanicInfo};

extern "C" {
    fn main();
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        main();
    }
    #[allow(clippy::empty_loop)]
    loop {}
}

#[panic_handler]
pub fn panic(_: &PanicInfo) -> ! {
    loop {}
}

pub fn write_char(c: char) {
    syscall_1(0, c as usize);
}

fn syscall_1(nr: usize, arg1: usize) -> usize {
    let ret: usize;
    unsafe {
        asm!("ecall",
            in("a7") nr,
            in("a0") arg1,
            lateout("a0") ret,
        );
    }
    ret
}

pub fn wait() {
    for _ in 0..100000000 {
        unsafe {
            asm!("nop");
        }
    }
}
