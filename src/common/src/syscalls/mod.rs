extern crate alloc;
extern crate macros;

use macros::syscalls;

pub mod trap_frame;
pub mod userpointer;

pub const SYSCALL_SUCCESS: isize = 0;

syscalls!(
    WRITE_CHAR(c: u8);
);
