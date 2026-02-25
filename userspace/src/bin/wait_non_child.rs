fn main() {
    let mut status: i32 = 0;
    let ret = unsafe { raw_waitpid(1, &mut status, 0) };
    if ret < 0 {
        println!("NotAChild");
    } else {
        println!("Unexpected: waitpid returned {ret}");
    }
}

unsafe fn raw_waitpid(pid: i32, status: *mut i32, options: i32) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!("ecall",
            in("a7") 260_usize,  // __NR_wait4
            in("a0") pid,
            in("a1") status,
            in("a2") options,
            in("a3") 0_usize,    // rusage
            lateout("a0") ret,
        );
    }
    ret
}
