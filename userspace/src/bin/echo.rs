#![no_std]
#![no_main]

use userspace::{args, print, println};

extern crate userspace;

#[unsafe(no_mangle)]
fn main() {
    let args = args();
    for (index, arg) in args.skip(1).enumerate() {
        if index > 0 {
            print!(" ");
        }
        print!("{arg}");
    }
    println!("");
}
