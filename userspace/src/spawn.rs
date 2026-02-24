use std::ffi::CString;

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

    #[allow(deprecated)]
    let pid = unsafe { libc::vfork() };

    if pid < 0 {
        return Err(pid);
    }

    if pid == 0 {
        // Child: call execve
        let ret = unsafe { libc::execve(path.as_ptr(), argv_ptrs.as_ptr(), envp.as_ptr()) };
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
        unsafe { libc::waitpid(pid, core::ptr::null_mut(), 0) };
        return Err(errno);
    }

    Ok(pid)
}
