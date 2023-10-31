#![no_std]
#![no_main]

extern crate userspace;

// static mut X: i32 = 0;

#[no_mangle]
fn main() {
    loop {}
    // loop {
    //     unsafe {
    //         X += 1;
    //     }
    // }
}
