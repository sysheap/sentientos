use userspace::spawn::spawn;

extern crate userspace;

fn main() {
    println!("init process started");
    println!("starting shell");
    let shell_pid = spawn("sosh", &[]).expect("Failed to spawn shell");
    unsafe {
        libc::waitpid(shell_pid, core::ptr::null_mut(), 0);
    }
    println!("Initial shell has exited...");
}
