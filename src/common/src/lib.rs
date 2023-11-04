#![no_std]

extern crate alloc;
use alloc::vec::Vec;

pub mod syscalls;

struct Foo;

impl syscalls::kernel::Syscalls for Foo {
    #[allow(non_snake_case)]
    fn WRITE_CHAR(c: u8) -> isize {
        todo!()
    }
}
