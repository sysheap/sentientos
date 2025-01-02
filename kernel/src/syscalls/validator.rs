use core::ops::{Deref, DerefMut};

use common::{
    constructable::Constructable,
    net::UDPDescriptor,
    syscalls::{SysSocketError, ValidationError},
    unwrap_or_return,
};

use crate::net::sockets::SharedAssignedSocket;

use super::handler::SyscallHandler;

pub struct UserspaceArgument<T> {
    inner: T,
}

impl<T> Constructable<T> for UserspaceArgument<T> {
    fn new(inner: T) -> Self {
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
        let start = self.inner.as_ptr();
        let len = self.inner.len();
        let ptr = handler.current_process().with_lock(|p| {
            let pt = p.get_page_table();
            if !pt.is_valid_userspace_fat_ptr(start, len, false) {
                return None;
            }
            pt.translate_userspace_address_to_physical_address(start)
        });

        if let Some(ptr) = ptr {
            // SAFETY: We validated the pointer above
            unsafe { Ok(core::str::from_raw_parts(ptr, len)) }
        } else {
            Err(ValidationError::InvalidPtr)
        }
    }
}

impl<'a> Validatable<&'a [u8]> for UserspaceArgument<&'a [u8]> {
    type Error = ValidationError;
    fn validate(self, handler: &mut SyscallHandler) -> Result<&'a [u8], Self::Error> {
        let start = self.inner.as_ptr();
        let len = self.inner.len();
        let ptr = handler.current_process().with_lock(|p| {
            let pt = p.get_page_table();
            if !pt.is_valid_userspace_fat_ptr(start, len, false) {
                return None;
            }
            pt.translate_userspace_address_to_physical_address(start)
        });

        if let Some(ptr) = ptr {
            // SAFETY: We validated the pointer above
            unsafe { Ok(core::slice::from_raw_parts(ptr, len)) }
        } else {
            Err(ValidationError::InvalidPtr)
        }
    }
}

impl<'a> Validatable<&'a mut [u8]> for UserspaceArgument<&'a mut [u8]> {
    type Error = ValidationError;
    fn validate(self, handler: &mut SyscallHandler) -> Result<&'a mut [u8], Self::Error> {
        let start = self.inner.as_mut_ptr();
        let len = self.inner.len();
        let ptr = handler.current_process().with_lock(|p| {
            let pt = p.get_page_table();
            if !pt.is_valid_userspace_fat_ptr(start, len, false) {
                return None;
            }
            pt.translate_userspace_address_to_physical_address(start)
        });

        if let Some(ptr) = ptr {
            // SAFETY: We validated the pointer above
            unsafe { Ok(core::slice::from_raw_parts_mut(ptr, len)) }
        } else {
            Err(ValidationError::InvalidPtr)
        }
    }
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
