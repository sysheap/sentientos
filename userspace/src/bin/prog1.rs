#![no_std]
#![no_main]

extern crate userspace;

#[no_mangle]
fn main() {
    let mut _x = 42;
    loop {
        _x += 1;
    }
}
