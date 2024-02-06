use crate::ecall;
use crate::syscalls;

use self::syscall_argument::SyscallArgument;

mod ecall;
mod macros;
mod syscall_argument;
pub mod trap_frame;
pub mod userspace_argument;

use self::ecall::*;
use self::userspace_argument::UserspaceArgument;

pub const SYSCALL_SUCCESS: isize = 0;
pub const SYSCALL_WAIT: isize = -1;
pub const SYSCALL_INVALID_PTR: isize = -2;
pub const SYSCALL_INVALID_PROGRAM: isize = -3;
pub const SYSCALL_INVALID_PID: isize = -4;

syscalls!(
    sys_write_char(c: char) -> ();
    sys_read_char() -> isize;
    sys_exit(status: isize) -> ();
    // TODO: Implement slice as argument using a wrapper
    sys_execute(name: &u8, length: usize) -> isize;
    sys_wait(pid: u64) -> isize;
    sys_mmap_pages(number_of_pages: usize) -> *mut u8;
);
