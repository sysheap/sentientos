extern crate macros;

use macros::syscalls;

pub mod trap_frame;
pub mod userpointer;

pub const SYSCALL_SUCCESS: isize = 0;
pub const SYSCALL_WAIT: isize = -1;

syscalls!(
    WRITE_CHAR(c: u8);
    READ_CHAR();
    EXIT(status: isize);
);
