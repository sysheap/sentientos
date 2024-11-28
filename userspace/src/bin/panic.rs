#![no_std]
#![no_main]

use common::syscalls::sys_panic;
use userspace::println;

extern crate userspace;

#[unsafe(no_mangle)]
fn main() {
    println!("Hello from Panic! Triggering kernel panic");
    sys_panic();
}
