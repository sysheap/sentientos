#![no_std]
#![no_main]

extern crate userspace;

// static mut X: i32 = 0;

#[no_mangle]
fn main() {
    #[allow(clippy::empty_loop)]
    loop {}
    // loop {
    //     unsafe {
    //         X += 1;
    //     }
    // }
}
