use crate::{ecall, net::UDPDescriptor, syscalls};

use self::syscall_argument::{SyscallArgument, SyscallReturnArgument};

mod ecall;
mod macros;
mod syscall_argument;
pub mod trap_frame;
pub mod userspace_argument;

use self::{ecall::*, userspace_argument::UserspaceArgument};

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

#[derive(Debug)]
#[repr(usize)]
pub enum SysSocketError {
    PortAlreadyUsed,
    InvalidPtr,
    InvalidDescriptor,
    NoReceiveIPYet,
}

syscalls!(
    sys_write_char(c: char) -> ();
    sys_read_input() -> Option<u8>;
    sys_exit(status: isize) -> ();
    // TODO: Implement slice as argument using a wrapper
    sys_execute(name: &u8, length: usize) -> Result<u64, SysExecuteError>;
    sys_wait(pid: u64) -> Result<(), SysWaitError>;
    sys_mmap_pages(number_of_pages: usize) -> *mut u8;
    sys_open_udp_socket(port: u16) -> Result<UDPDescriptor, SysSocketError>;
    sys_write_back_udp_socket(descriptor: UDPDescriptor, buffer: &u8, length: usize) -> Result<usize, SysSocketError>;
    sys_read_udp_socket(descriptor: UDPDescriptor, buffer: &mut u8, length: usize) -> Result<usize, SysSocketError>;
);
