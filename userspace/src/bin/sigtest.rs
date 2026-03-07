extern crate userspace;

use std::sync::atomic::{AtomicI32, Ordering};

static CAUGHT_SIGNAL: AtomicI32 = AtomicI32::new(0);

unsafe extern "C" fn handler(sig: i32) {
    CAUGHT_SIGNAL.store(sig, Ordering::SeqCst);
}

unsafe extern "C" {
    fn getpid() -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("self-kill");

    match mode {
        "self-kill" => test_self_kill(),
        "ignore" => test_ignore(),
        "wait-for-signal" => test_wait_for_signal(),
        other => {
            eprintln!("unknown mode: {other}");
            std::process::exit(1);
        }
    }
}

fn test_self_kill() {
    install_handler(2); // SIGINT
    let pid = unsafe { getpid() };
    let ret = unsafe { kill(pid, 2) };
    assert_eq!(ret, 0, "kill failed");
    let caught = CAUGHT_SIGNAL.load(Ordering::SeqCst);
    assert_eq!(caught, 2, "expected SIGINT (2), got {caught}");
    println!("caught signal {caught}");
    println!("OK");
}

fn test_ignore() {
    install_ignore(2); // SIGINT
    println!("waiting");
    // Sleep while the test sends Ctrl+C — with SIG_IGN it should be ignored.
    std::thread::sleep(std::time::Duration::from_secs(2));
    println!("OK");
}

fn test_wait_for_signal() {
    install_handler(2); // SIGINT
    println!("waiting");
    // Busy-wait for a signal
    while CAUGHT_SIGNAL.load(Ordering::SeqCst) == 0 {
        core::hint::spin_loop();
    }
    let caught = CAUGHT_SIGNAL.load(Ordering::SeqCst);
    println!("caught signal {caught}");
    println!("OK");
}

fn install_handler(sig: i32) {
    // sigaction struct layout for RISC-V Linux (no sa_restorer):
    //   sa_handler: 8 bytes (function pointer)
    //   sa_flags:   8 bytes
    //   sa_mask:    8 bytes (sigset_t)
    let sa: [u64; 3] = [handler as *const () as u64, 0, 0];
    let ret = unsafe { libc_rt_sigaction(sig, sa.as_ptr(), core::ptr::null_mut(), 8) };
    assert_eq!(ret, 0, "rt_sigaction failed with {ret}");
}

fn install_ignore(sig: i32) {
    let sa: [u64; 3] = [1, 0, 0];
    let ret = unsafe { libc_rt_sigaction(sig, sa.as_ptr(), core::ptr::null_mut(), 8) };
    assert_eq!(ret, 0, "rt_sigaction failed with {ret}");
}

unsafe extern "C" {
    #[link_name = "syscall"]
    fn libc_syscall(num: i64, ...) -> i64;
}

unsafe fn libc_rt_sigaction(sig: i32, act: *const u64, oact: *mut u64, sigsetsize: usize) -> i64 {
    // rt_sigaction is syscall 134 on RISC-V
    unsafe { libc_syscall(134, sig, act, oact, sigsetsize) }
}
