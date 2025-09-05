use common::syscalls::{sys_execute, sys_wait};
use userspace::println;

use std::vec::Vec;

extern crate alloc;
extern crate userspace;

const INSTANCES: usize = 32;

fn main() {
    println!("Starting loop {INSTANCES} times");
    let mut pids = Vec::with_capacity(INSTANCES);
    for _ in 0..INSTANCES {
        let pid = sys_execute("loop", &[]).expect("Process must be successfully startable");
        pids.push(pid);
    }

    for pid in pids {
        let _ = sys_wait(pid);
    }

    println!("Done!");
}
