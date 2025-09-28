use crate::processes::{process::ProcessRef, userspace_ptr::UserspacePtr};
use alloc::{string::String, vec::Vec};
use common::pointer::Pointer;
use core::marker::PhantomData;
use headers::errno::Errno;

pub struct LinuxUserspaceArg<T> {
    arg: usize,
    process: ProcessRef,
    phantom: PhantomData<T>,
}

impl<T> LinuxUserspaceArg<T> {
    pub fn new(arg: usize, process: ProcessRef) -> Self {
        Self {
            arg,
            process,
            phantom: PhantomData,
        }
    }
}

impl LinuxUserspaceArg<*const u8> {
    pub fn validate_str(&self, len: usize) -> Result<String, Errno> {
        self.process
            .with_lock(|p| p.read_userspace_str(&self.into(), len))
    }
}

impl<T> LinuxUserspaceArg<*const T> {
    pub fn validate_ptr(&self) -> Result<T, Errno> {
        self.process
            .with_lock(|p| p.read_userspace_ptr(&self.into()))
    }
}

impl<T> LinuxUserspaceArg<Option<*const T>> {
    pub fn validate_ptr(&self) -> Result<Option<T>, Errno> {
        if self.arg == 0 {
            return Ok(None);
        }
        self.process
            .with_lock(|p| p.read_userspace_ptr(&self.into()))
            .map(|r| Some(r))
    }
}

impl<T: Clone> LinuxUserspaceArg<*mut T> {
    pub fn validate_slice(&self, len: usize) -> Result<Vec<T>, Errno> {
        self.process
            .with_lock(|p| p.read_userspace_slice(&self.into(), len))
    }
}

impl<T: Clone> LinuxUserspaceArg<Option<*mut T>> {
    pub fn write_if_not_none(&self, value: T) -> Result<Option<()>, Errno> {
        if self.arg == 0 {
            return Ok(None);
        }
        self.process
            .with_lock(|p| p.write_userspace_ptr(&self.into(), value))?;
        Ok(Some(()))
    }
}

impl<PTR: Pointer> From<&LinuxUserspaceArg<PTR>> for UserspacePtr<PTR> {
    fn from(value: &LinuxUserspaceArg<PTR>) -> Self {
        Self::new(PTR::as_pointer(value.arg))
    }
}

impl<PTR: Pointer> From<&LinuxUserspaceArg<Option<PTR>>> for UserspacePtr<PTR> {
    fn from(value: &LinuxUserspaceArg<Option<PTR>>) -> Self {
        Self::new(PTR::as_pointer(value.arg))
    }
}

impl<T> From<&LinuxUserspaceArg<*mut T>> for UserspacePtr<*const T> {
    fn from(value: &LinuxUserspaceArg<*mut T>) -> Self {
        Self::new(value.arg as *const T)
    }
}
