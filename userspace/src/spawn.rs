use std::ffi::CString;

pub fn spawn(program: &str, args: &[&str]) -> Result<i32, i32> {
    let path = CString::new(program).expect("program name must not contain NUL");

    let mut argv_cstrings: Vec<CString> = Vec::with_capacity(args.len() + 1);
    argv_cstrings.push(path.clone());
    for arg in args {
        argv_cstrings.push(CString::new(*arg).expect("arg must not contain NUL"));
    }

    let mut argv_ptrs: Vec<*mut libc::c_char> = argv_cstrings
        .iter()
        .map(|s| s.as_ptr() as *mut libc::c_char)
        .collect();
    argv_ptrs.push(core::ptr::null_mut());

    let envp: [*mut libc::c_char; 1] = [core::ptr::null_mut()];

    let mut pid: libc::pid_t = 0;
    let ret = unsafe {
        libc::posix_spawn(
            &mut pid,
            path.as_ptr(),
            core::ptr::null(),
            core::ptr::null(),
            argv_ptrs.as_ptr(),
            envp.as_ptr(),
        )
    };

    if ret != 0 {
        Err(ret)
    } else {
        Ok(pid)
    }
}
