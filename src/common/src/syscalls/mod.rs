extern crate macros;

use macros::syscalls;

pub mod trap_frame;
pub mod userpointer;

pub const SYSCALL_SUCCESS: isize = 0;
pub const SYSCALL_WAIT: isize = -1;
pub const SYSCALL_INVALID_PTR: isize = -2;
pub const SYSCALL_INVALID_PROGRAM: isize = -3;
pub const SYSCALL_INVALID_PID: isize = -4;

syscalls!(
    WRITE_CHAR(c: u8);
    READ_CHAR();
    EXIT(status: isize);
    EXECUTE(name: &u8, length: usize);
    WAIT(pid: u64);
    MMAP_PAGES(number_of_pages: usize);
);
