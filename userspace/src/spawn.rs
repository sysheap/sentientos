use std::ffi::CString;

const SYSCALL_NR_CLONE: usize = 220;
const SYSCALL_NR_EXECVE: usize = 221;
const CLONE_VM: usize = 0x100;
const CLONE_VFORK: usize = 0x4000;
const SIGCHLD: usize = 17;

unsafe fn syscall5(nr: usize, a0: usize, a1: usize, a2: usize, a3: usize, a4: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") a0 => ret,
            in("a1") a1,
            in("a2") a2,
            in("a3") a3,
            in("a4") a4,
            in("a7") nr,
        );
    }
    ret
}

unsafe fn syscall3(nr: usize, a0: usize, a1: usize, a2: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") a0 => ret,
            in("a1") a1,
            in("a2") a2,
            in("a7") nr,
        );
    }
    ret
}

pub fn spawn(program: &str, args: &[&str]) -> Result<i32, i32> {
    let path = CString::new(program).expect("program name must not contain NUL");

    let mut argv_cstrings: Vec<CString> = Vec::with_capacity(args.len() + 1);
    argv_cstrings.push(path.clone());
    for arg in args {
        argv_cstrings.push(CString::new(*arg).expect("arg must not contain NUL"));
    }

    let mut argv_ptrs: Vec<*const libc::c_char> =
        argv_cstrings.iter().map(|s| s.as_ptr()).collect();
    argv_ptrs.push(core::ptr::null());

    let envp: [*const libc::c_char; 1] = [core::ptr::null()];

    // Shared between parent and child via CLONE_VM. CLONE_VFORK guarantees
    // the parent reads this only after the child has called execve or exited.
    let mut execve_errno: i32 = 0;

    let flags = CLONE_VM | CLONE_VFORK | SIGCHLD;
    let pid = unsafe { syscall5(SYSCALL_NR_CLONE, flags, 0, 0, 0, 0) };

    if pid < 0 {
        return Err(pid as i32);
    }

    if pid == 0 {
        // Child: call execve
        let ret = unsafe {
            syscall3(
                SYSCALL_NR_EXECVE,
                path.as_ptr() as usize,
                argv_ptrs.as_ptr() as usize,
                envp.as_ptr() as usize,
            )
        };
        // execve only returns on error - signal parent via shared variable
        unsafe {
            core::ptr::write_volatile(&mut execve_errno, ret as i32);
            libc::_exit(127);
        }
    }

    // CLONE_VFORK guarantees child has called execve (success) or exited (failure)
    let errno = unsafe { core::ptr::read_volatile(&execve_errno) };
    if errno != 0 {
        // Reap zombie child
        unsafe { libc::waitpid(pid as i32, core::ptr::null_mut(), 0) };
        return Err(errno);
    }

    Ok(pid as i32)
}
