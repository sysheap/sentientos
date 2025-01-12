use crate::impl_from_to;

#[derive(Debug)]
pub enum LoaderError {
    StackToSmall,
}

#[derive(Debug)]
pub enum SchedulerError {
    InvalidProgramName,
    LoaderError(LoaderError),
}

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
    SchedulerError(SchedulerError),
}

#[derive(Debug)]
#[repr(usize)]
pub enum SysArgError {
    InvalidIndex,
    ValidationError(ValidationError),
    SpaceTooSmall,
}

#[derive(Debug)]
pub enum SysSocketError {
    PortAlreadyUsed,
    ValidationError(ValidationError),
    InvalidDescriptor,
    NoReceiveIPYet,
}

impl_from_to!(ValidationError, SysExecuteError);
impl_from_to!(ValidationError, SysSocketError);
impl_from_to!(ValidationError, SysArgError);
impl_from_to!(LoaderError, SchedulerError);
impl_from_to!(SchedulerError, SysExecuteError);
