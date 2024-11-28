#![no_std]
#![no_main]

use userspace::{println, util::wait};

extern crate userspace;

#[unsafe(no_mangle)]
fn main() {
    println!("Hello from Loop");
    for i in 0..10 {
        println!("Looping... {}", i);
        wait(1000000000);
    }
}
