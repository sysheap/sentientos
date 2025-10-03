use common::syscalls::{sys_execute, sys_wait};

extern crate userspace;

fn main() {
    println!("init process started");
    println!("starting shell");
    let shell_name = "sesh";
    let shell_pid = sys_execute(shell_name, &[]).unwrap();
    sys_wait(shell_pid).unwrap();
    println!("Initial shell has exited...");
}
