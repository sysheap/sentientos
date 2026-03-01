use std::{env, process::exit, thread, time::Duration};

const DEFAULT_INSTANCES: usize = 32;

fn main() {
    let args: Vec<String> = env::args().collect();

    let instances: usize = if args.len() > 1 {
        match args[1].parse() {
            Ok(n) if n > 0 => n,
            _ => {
                eprintln!("Usage: {} [count]", args[0]);
                eprintln!(
                    "  count: number of threads to spawn (default: {})",
                    DEFAULT_INSTANCES
                );
                exit(1);
            }
        }
    } else {
        DEFAULT_INSTANCES
    };

    println!("Starting loop {} times", instances);
    let mut handles = Vec::with_capacity(instances);
    for _ in 0..instances {
        let handle = thread::spawn(|| {
            for i in 0..5 {
                println!("Looping... {}", i);
                thread::sleep(Duration::from_secs(1));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread must join successfully");
    }

    println!("Done!");
}
