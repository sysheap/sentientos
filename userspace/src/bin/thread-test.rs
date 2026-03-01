use std::thread;

fn main() {
    println!("Before spawn");
    let handle = thread::spawn(|| {
        println!("Hello from thread!");
    });
    println!("After spawn, before join");
    handle.join().expect("join failed");
    println!("Done!");
}
