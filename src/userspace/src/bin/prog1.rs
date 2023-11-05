#![no_std]
#![no_main]

use common::syscalls::userspace::WRITE_CHAR;
use userspace::util::wait;

extern crate userspace;

#[no_mangle]
fn main() {
    loop {
        WRITE_CHAR(b'a');
        wait();
    }
}
