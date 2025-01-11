use core::ops::{Deref, DerefMut};

use common::{
    constructable::Constructable,
    net::UDPDescriptor,
    numbers::Number,
    pointer::{FatPointer, Pointer},
    syscalls::{syscall_argument::SyscallArgument, SysSocketError, ValidationError},
    unwrap_or_return,
};

use crate::net::sockets::SharedAssignedSocket;

use super::handler::SyscallHandler;

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

impl<'a, T: Number> Validatable<&'a [T]> for UserspaceArgument<&'a [T]> {
    type Error = ValidationError;

    fn validate(self, handler: &mut SyscallHandler) -> Result<&'a [T], Self::Error> {
        let ptr = validate_and_translate_slice_ptr(self.inner, handler)?;

        // SAFETY: we validated the pointer above
        unsafe { Ok(core::slice::from_raw_parts(ptr, self.inner.len())) }
    }
}

impl<'a, T: Number> Validatable<&'a mut [T]> for UserspaceArgument<&'a mut [T]> {
    type Error = ValidationError;

    fn validate(self, handler: &mut SyscallHandler) -> Result<&'a mut [T], Self::Error> {
        let ptr = validate_and_translate_slice_ptr(self.inner, handler)?;

        // SAFETY: we validated the pointer above
        unsafe { Ok(core::slice::from_raw_parts_mut(ptr, self.inner.len())) }
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
            if !pt.is_valid_userspace_fat_ptr(ptr, len, false) {
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
