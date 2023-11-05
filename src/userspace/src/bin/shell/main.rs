#![no_std]
#![no_main]

use userspace::{wait, write_char};

extern crate userspace;

#[no_mangle]
fn main() {
    write_char('s');
    write_char('h');
    write_char('e');
    write_char('l');
    write_char('l');
    loop {}
}
