#![no_std]
#![no_main]

use userspace::{wait, write_char};

extern crate userspace;

#[no_mangle]
fn main() {
    loop {
        write_char('a');
        wait();
    }
}
