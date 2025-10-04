use super::handler::SyscallHandler;
use crate::net::sockets::SharedAssignedSocket;
use alloc::vec::Vec;
use common::{
    constructable::Constructable,
    errors::{SysSocketError, ValidationError},
    net::UDPDescriptor,
    pid::Tid,
    pointer::{FatPointer, Pointer},
    syscalls::syscall_argument::SyscallArgument,
    unwrap_or_return,
};
use core::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct UserspaceArgument<T: SyscallArgument> {
    inner: T::Converted,
}

impl<T: SyscallArgument> Constructable<T::Converted> for UserspaceArgument<T> {
    fn new(inner: T::Converted) -> Self {
        UserspaceArgument { inner }
    }
}

pub trait Validatable<T: Sized> {
    type Error;

    fn validate(self, handler: &mut SyscallHandler) -> Result<T, Self::Error>;
}

impl Validatable<SharedAssignedSocket> for UserspaceArgument<UDPDescriptor> {
    type Error = SysSocketError;

    fn validate(self, handler: &mut SyscallHandler) -> Result<SharedAssignedSocket, Self::Error> {
        let socket = unwrap_or_return!(
            handler
                .current_process()
                .with_lock(|mut p| p.get_shared_udp_socket(self.inner).cloned()),
            Err(SysSocketError::InvalidDescriptor)
        );
        Ok(socket)
    }
}

impl<'a> Validatable<&'a str> for UserspaceArgument<&'a str> {
    type Error = ValidationError;

    fn validate(self, handler: &mut SyscallHandler) -> Result<&'a str, Self::Error> {
        let ptr = validate_and_translate_slice_ptr(self.inner, handler)?;

        // SAFETY: we validated the pointer above
        unsafe { Ok(core::str::from_raw_parts(ptr, self.inner.len())) }
    }
}

impl<'a> Validatable<&'a [u8]> for UserspaceArgument<&'a [u8]> {
    type Error = ValidationError;

    fn validate(self, handler: &mut SyscallHandler) -> Result<&'a [u8], Self::Error> {
        let ptr = validate_and_translate_slice_ptr(self.inner, handler)?;

        // SAFETY: we validated the pointer above
        unsafe { Ok(core::slice::from_raw_parts(ptr, self.inner.len())) }
    }
}

impl<'a> Validatable<&'a mut [u8]> for UserspaceArgument<&'a mut [u8]> {
    type Error = ValidationError;

    fn validate(self, handler: &mut SyscallHandler) -> Result<&'a mut [u8], Self::Error> {
        let ptr = validate_and_translate_slice_ptr(self.inner, handler)?;

        // SAFETY: we validated the pointer above
        unsafe { Ok(core::slice::from_raw_parts_mut(ptr, self.inner.len())) }
    }
}

impl<'a> Validatable<Vec<&'a str>> for UserspaceArgument<&'a [&'a str]> {
    type Error = ValidationError;

    fn validate(self, handler: &mut SyscallHandler) -> Result<Vec<&'a str>, Self::Error> {
        // If we have zero length the pointer is not even allocated
        if self.inner.len() == 0 {
            return Ok(Vec::new());
        }

        let outer_slice_ptr = validate_and_translate_slice_ptr(self.inner, handler)?;

        // SAFETY: we validated the pointer above
        let outer_slice = unsafe { core::slice::from_raw_parts(outer_slice_ptr, self.inner.len()) };

        let mut result = Vec::with_capacity(outer_slice.len());

        for fat_ptr in outer_slice {
            let ptr = validate_and_translate_slice_ptr(*fat_ptr, handler)?;
            // SAFETY: We just validated the pointer above
            unsafe {
                result.push(core::str::from_raw_parts(ptr, fat_ptr.len()));
            }
        }

        Ok(result)
    }
}

fn validate_and_translate_slice_ptr<PTR: Pointer>(
    fat_pointer: FatPointer<PTR>,
    handler: &mut SyscallHandler,
) -> Result<PTR, ValidationError> {
    let ptr = fat_pointer.ptr();
    let len = fat_pointer.len();

    handler
        .current_process()
        .with_lock(|p| {
            let pt = p.get_page_table();
            if !pt.is_valid_userspace_fat_ptr(ptr, len, PTR::WRITABLE) {
                return None;
            }
            pt.translate_userspace_address_to_physical_address(ptr)
        })
        .ok_or(ValidationError::InvalidPtr)
}

macro_rules! simple_type {
    ($ty:ty) => {
        impl Deref for UserspaceArgument<$ty> {
            type Target = $ty;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl DerefMut for UserspaceArgument<$ty> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }
    };
}

simple_type!(char);

simple_type!(u8);
simple_type!(u16);
simple_type!(u32);
simple_type!(u64);
simple_type!(usize);

simple_type!(i8);
simple_type!(i16);
simple_type!(i32);
simple_type!(i64);
simple_type!(isize);

simple_type!(Tid);
