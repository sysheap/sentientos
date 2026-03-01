use std::{env, process::exit};
use userspace::spawn::spawn;

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
    let mut children = Vec::with_capacity(instances);
    for _ in 0..instances {
        let child = spawn("loop", &[]).expect("Process must be successfully startable");
        children.push(child);
    }

    for mut child in children {
        let _ = child.wait();
    }

    println!("Done!");
}
