use common::syscalls::sys_execute;

extern crate userspace;

fn main() {
    println!("init process started");
    println!("starting shell");
    let shell_name = "sesh";
    let shell_pid = sys_execute(shell_name, &[]).unwrap();
    unsafe {
        libc::waitpid(shell_pid.0 as i32, core::ptr::null_mut(), 0);
    }
    println!("Initial shell has exited...");
}
