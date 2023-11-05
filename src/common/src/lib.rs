#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use syscalls::{Userpointer, UserpointerMut};

pub mod syscalls;

struct Foo;

impl syscalls::kernel::Syscalls for Foo {
    #[allow(non_snake_case)]
    fn WRITE_CHAR(c: u8) -> isize {
        todo!()
    }

    #[allow(non_snake_case)]
    fn SHARE_VEC(vec: UserpointerMut<Vec<u8>>, additional_data: usize) -> isize {
        todo!()
    }
}
