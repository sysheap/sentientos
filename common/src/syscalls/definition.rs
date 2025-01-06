use crate::{net::UDPDescriptor, scalar_enum};

use super::macros::syscalls;

#[derive(Debug)]
pub enum ValidationError {
    InvalidPtr,
}

#[derive(Debug)]
#[repr(usize)]
pub enum SysWaitError {
    InvalidPid,
}

#[derive(Debug)]
#[repr(usize)]
pub enum SysExecuteError {
    InvalidProgram,
    ValidationError(ValidationError),
}

#[derive(Debug)]
#[repr(usize)]
pub enum SysArgError {
    InvalidIndex,
    ValidationError(ValidationError),
    SpaceTooSmall,
}

#[derive(Debug)]
#[repr(usize)]
pub enum SysSocketError {
    PortAlreadyUsed,
    ValidationError(ValidationError),
    InvalidDescriptor,
    NoReceiveIPYet,
}

macro_rules! impl_from_validation_error {
    ($ty:ty) => {
        impl From<ValidationError> for $ty {
            fn from(value: ValidationError) -> Self {
                Self::ValidationError(value)
            }
        }
    };
}

impl_from_validation_error!(SysExecuteError);
impl_from_validation_error!(SysSocketError);
impl_from_validation_error!(SysArgError);

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
    sys_write<'a>(s: &'a str) -> Result<(), ValidationError>;
    sys_read_input() -> Option<u8>;
    sys_read_input_wait() -> u8;
    sys_exit(status: isize) -> ();
    sys_execute<'a>(name: &'a str) -> Result<u64, SysExecuteError>;
    sys_execute_add_arg<'a>(arg: &'a str) -> Result<(), ValidationError>;
    sys_execute_arg_clear() -> ();
    sys_execute_number_of_args() -> usize;
    sys_execute_get_arg<'a>(index: usize, buffer: &'a mut [u8]) -> Result<usize, SysArgError>;
    sys_wait(pid: u64) -> Result<(), SysWaitError>;
    sys_mmap_pages(number_of_pages: usize) -> *mut u8;
    sys_open_udp_socket(port: u16) -> Result<UDPDescriptor, SysSocketError>;
    sys_write_back_udp_socket<'a>(descriptor: UDPDescriptor, buffer: &'a [u8]) -> Result<usize, SysSocketError>;
    sys_read_udp_socket<'a>(descriptor: UDPDescriptor, buffer: &'a mut [u8]) -> Result<usize, SysSocketError>;
    sys_panic() -> ();
    sys_print_programs() -> ();
);
