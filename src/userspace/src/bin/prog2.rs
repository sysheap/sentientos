#![no_std]
#![no_main]

use common::syscalls::userspace::WRITE_CHAR;
use userspace::wait;

extern crate userspace;

#[no_mangle]
fn main() {
    loop {
        WRITE_CHAR(b'b');
        wait();
    }
}
