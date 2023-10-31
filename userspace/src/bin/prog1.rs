#![no_std]
#![no_main]

extern crate userspace;

static mut _x: i32 = 0;

#[no_mangle]
fn main() {
    loop {
        unsafe {
            _x += 1;
        }
    }
}
