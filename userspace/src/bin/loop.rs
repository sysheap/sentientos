use userspace::{println, util::wait};

extern crate userspace;

fn main() {
    println!("Hello from Loop");
    for i in 0..10 {
        println!("Looping... {}", i);
        wait(100000000);
    }
}
