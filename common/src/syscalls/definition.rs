use crate::{
    errors::{SysExecuteError, SysSocketError, SysWaitError},
    net::UDPDescriptor,
    pid::Tid,
    scalar_enum,
};

use super::macros::syscalls;

scalar_enum! {
    #[repr(usize)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum SyscallStatus {
        Success,
        InvalidSyscallNumber,
        InvalidArgPtr,
        InvalidRetPtr,
    }
}

syscalls!(
    sys_read_input() -> Option<u8>;
    sys_exit(status: isize) -> ();
    sys_execute<'a>(name: &'a str, args: &'a [&'a str]) -> Result<Tid, SysExecuteError>;
    sys_wait(tid: Tid) -> Result<(), SysWaitError>;
    sys_mmap_pages(number_of_pages: usize) -> *mut u8;
    sys_open_udp_socket(port: u16) -> Result<UDPDescriptor, SysSocketError>;
    sys_write_back_udp_socket<'a>(descriptor: UDPDescriptor, buffer: &'a [u8]) -> Result<usize, SysSocketError>;
    sys_read_udp_socket<'a>(descriptor: UDPDescriptor, buffer: &'a mut [u8]) -> Result<usize, SysSocketError>;
    sys_panic() -> ();
    sys_print_programs() -> ();
);
