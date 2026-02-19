use crate::{errors::SysExecuteError, pid::Tid, scalar_enum};

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
);
