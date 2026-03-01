use userspace::spawn::spawn;

fn main() {
    println!("init process started");
    println!("starting shell");
    let mut child = spawn("sosh", &[]).expect("Failed to spawn shell");
    child.wait().expect("Failed to wait for shell");
    println!("Initial shell has exited...");
}
