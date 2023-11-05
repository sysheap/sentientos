#![no_std]
#![no_main]

use common::syscalls::userspace::WRITE_CHAR;

extern crate userspace;

#[no_mangle]
fn main() {
    WRITE_CHAR(b's');
    WRITE_CHAR(b'h');
    WRITE_CHAR(b'e');
    WRITE_CHAR(b'l');
    WRITE_CHAR(b'l');
    loop {}
}
