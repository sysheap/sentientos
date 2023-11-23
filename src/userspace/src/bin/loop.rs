#![no_std]
#![no_main]

use userspace::{println, util::wait};

extern crate userspace;

#[no_mangle]
fn main() {
    println!("Hello from Loop");
    let mut counter: usize = 0;
    loop {
        println!("Looping... {}", counter);
        counter += 1;
        wait(1000000000);
    }
}
