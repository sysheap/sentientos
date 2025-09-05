use std::env;

use userspace::{print, println};

extern crate userspace;

fn main() {
    for (index, arg) in env::args().skip(1).enumerate() {
        if index > 0 {
            print!(" ");
        }
        print!("{arg}");
    }
    println!("");
}
