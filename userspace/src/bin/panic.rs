use common::syscalls::sys_panic;
use userspace::println;

extern crate userspace;

fn main() {
    println!("Hello from Panic! Triggering kernel panic");
    sys_panic();
}
