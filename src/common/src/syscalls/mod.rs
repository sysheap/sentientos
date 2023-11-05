extern crate alloc;
extern crate macros;

use macros::syscalls;

pub mod trap_frame;

pub struct Userpointer<T> {
    ptr: *const T,
}

impl<T> Userpointer<T> {
    pub fn new(ptr: *const T) -> Self {
        Self { ptr }
    }
}

pub struct UserpointerMut<T> {
    ptr: *mut T,
}

impl<T> UserpointerMut<T> {
    pub fn new(ptr: *mut T) -> Self {
        Self { ptr }
    }
}

syscalls!(
    extern crate alloc;

    use alloc::vec::Vec;

    WRITE_CHAR(c: u8);
    SHARE_VEC(vec: &mut Vec<u8>, additional_data: usize);
);
