use crate::{
    errors::{SysExecuteError, SysSocketError},
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
    sys_execute<'a>(name: &'a str, args: &'a [&'a str]) -> Result<Tid, SysExecuteError>;
    sys_open_udp_socket(port: u16) -> Result<UDPDescriptor, SysSocketError>;
    sys_write_back_udp_socket<'a>(descriptor: UDPDescriptor, buffer: &'a [u8]) -> Result<usize, SysSocketError>;
    sys_read_udp_socket<'a>(descriptor: UDPDescriptor, buffer: &'a mut [u8]) -> Result<usize, SysSocketError>;
);
