use std::{env, process::exit, thread::sleep, time::Duration};

use userspace::println;

fn main() {
    // Collect command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <seconds>", args[0]);
        exit(1);
    }

    // Parse the argument into an integer
    let seconds: u64 = match args[1].parse() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error: '{}' is not a valid number", args[1]);
            exit(1);
        }
    };

    println!("Sleeping for {} seconds...", seconds);
    sleep(Duration::from_secs(seconds));
    println!("Done!");
}
