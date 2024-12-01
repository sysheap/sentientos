#![no_std]
#![no_main]

use common::syscalls::sys_idle;

extern crate userspace;

#[unsafe(no_mangle)]
fn main() {
    loop {
        sys_idle();
    }
}
