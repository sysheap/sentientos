use std::{thread::sleep, time::Duration};

extern crate userspace;

fn main() {
    println!("Hello from Loop");
    for i in 0..5 {
        println!("Looping... {}", i);
        sleep(Duration::from_secs(1));
    }
}
