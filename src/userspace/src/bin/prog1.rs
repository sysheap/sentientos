#![no_std]
#![no_main]

use userspace::{println, util::wait};

extern crate userspace;

#[no_mangle]
fn main() {
    loop {
        println!("a");
        wait();
    }
}
