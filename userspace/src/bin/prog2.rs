#![no_std]
#![no_main]

use userspace::println;

extern crate userspace;

#[unsafe(no_mangle)]
fn main() {
    println!("Hello from Prog2");
}
