use crate::impl_from_to;

#[derive(Debug)]
pub enum LoaderError {
    StackToSmall,
}

#[derive(Debug)]
pub enum ValidationError {
    InvalidPtr,
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
    TooManyOpenFiles,
}

impl_from_to!(ValidationError, SysSocketError);
impl_from_to!(ValidationError, SysArgError);
