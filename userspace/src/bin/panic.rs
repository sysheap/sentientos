use common::syscalls::sys_panic;

extern crate userspace;

fn main() {
    println!("Hello from Panic! Triggering kernel panic");
    sys_panic();
}
