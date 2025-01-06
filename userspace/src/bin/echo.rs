#![no_std]
#![no_main]

use common::syscalls::{sys_execute_get_arg, sys_execute_number_of_args};
use userspace::{print, println};

extern crate userspace;

#[unsafe(no_mangle)]
fn main() {
    let args = sys_execute_number_of_args();
    for index in 0..args {
        let mut buf = [0u8; 1024];
        let len = sys_execute_get_arg(index, &mut buf).expect("Could not read arg");
        let s = core::str::from_utf8(&buf[0..len]).expect("Argument must be utf8");
        print!("{s} ");
    }
    println!("");
}
