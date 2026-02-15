extern crate userspace;

fn main() {
    let ret = unsafe { libc::waitpid(1, core::ptr::null_mut(), 0) };
    if ret == -1 {
        println!("NotAChild");
    } else {
        println!("Unexpected: waitpid returned {ret}");
    }
}
