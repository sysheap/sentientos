#![no_std]
#![no_main]

use userspace::println;

extern crate userspace;

#[no_mangle]
fn main() {
    println!("YaSH - Yet another Shell");
}
