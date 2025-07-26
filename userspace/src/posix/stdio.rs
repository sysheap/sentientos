use crate::print;
use core::{
    ffi::{c_int, c_size_t, c_void},
    ptr::slice_from_raw_parts,
    str,
};

use common::syscalls::sys_exit;
use newlib_bindings::{_ssize_t, impl_binding};

impl_binding! {
    fn _write(fd: c_int, buf: *const c_void, nbyte: c_size_t) -> _ssize_t {
        if buf.is_null() {
            return -1;
        }
        let length = match _ssize_t::try_from(nbyte) {
            Ok(length) => length,
            Err(_) => {
                return -1;
            }
        };
        let slice = slice_from_raw_parts(buf as *const u8, nbyte);
        let string = unsafe { str::from_utf8(&*slice) };

        match string {
            Ok(s) => print!("{s}"),
            Err(_) => return -1,
        };
        length
    }
}

impl_binding! {
    fn _exit(status: c_int) -> ! {
        sys_exit(status as isize);
        panic!();
    }
}
