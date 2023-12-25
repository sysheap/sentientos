#![no_std]
#![no_main]

use userspace::println;

extern crate userspace;

#[no_mangle]
fn main() {
    println!("init process started");
    println!("starting shell");
    let shell_name = "shell";
    let shell_pid =
        common::syscalls::userspace::EXECUTE(&shell_name.as_bytes()[0], shell_name.len());
    common::syscalls::userspace::WAIT(shell_pid as u64);
    println!("Initial shell has exited...");
}
