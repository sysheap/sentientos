use crate::ecall;
use crate::syscalls;

use self::syscall_argument::{SyscallArgument, SyscallReturnArgument};

mod ecall;
mod macros;
mod syscall_argument;
pub mod trap_frame;
pub mod userspace_argument;

use self::ecall::*;
use self::userspace_argument::UserspaceArgument;

#[derive(Debug)]
#[repr(usize)]
pub enum SysWaitError {
    InvalidPid,
}

#[derive(Debug)]
#[repr(usize)]
pub enum SysExecuteError {
    InvalidPtr,
    InvalidProgram,
}

syscalls!(
    sys_write_char(c: char) -> ();
    sys_read_input() -> Option<u8>;
    sys_exit(status: isize) -> ();
    // TODO: Implement slice as argument using a wrapper
    sys_execute(name: &u8, length: usize) -> Result<u64, SysExecuteError>;
    sys_wait(pid: u64) -> Result<(), SysWaitError>;
    sys_mmap_pages(number_of_pages: usize) -> *mut u8;
);
