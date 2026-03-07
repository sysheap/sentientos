unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
}

fn main() {
    let pid = unsafe { fork() };
    if pid == 0 {
        println!("child");
    } else if pid > 0 {
        let mut status: i32 = 0;
        unsafe { waitpid(pid, &mut status, 0) };
        println!("parent waited child={pid}");
    } else {
        println!("fork failed");
    }
}
