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

impl_from_to!(ValidationError, SysArgError);
