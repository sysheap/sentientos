use common::syscalls::sys_execute;

use std::{env, process::exit, vec::Vec};

extern crate alloc;
extern crate userspace;

const DEFAULT_INSTANCES: usize = 32;

fn main() {
    let args: Vec<String> = env::args().collect();

    let instances: usize = if args.len() > 1 {
        match args[1].parse() {
            Ok(n) if n > 0 => n,
            _ => {
                eprintln!("Usage: {} [count]", args[0]);
                eprintln!(
                    "  count: number of processes to spawn (default: {})",
                    DEFAULT_INSTANCES
                );
                exit(1);
            }
        }
    } else {
        DEFAULT_INSTANCES
    };

    println!("Starting loop {} times", instances);
    let mut pids = Vec::with_capacity(instances);
    for _ in 0..instances {
        let pid = sys_execute("loop", &[]).expect("Process must be successfully startable");
        pids.push(pid);
    }

    for pid in pids {
        unsafe {
            libc::waitpid(pid.as_u64() as i32, core::ptr::null_mut(), 0);
        }
    }

    println!("Done!");
}
