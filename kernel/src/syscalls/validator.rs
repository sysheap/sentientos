use core::ops::{Deref, DerefMut};

use common::{
    constructable::Constructable,
    net::UDPDescriptor,
    pointer::{FatPointer, Pointer},
    ref_conversion::RefToPointer,
    syscalls::{SysSocketError, ValidationError},
    unwrap_or_return,
};

use crate::net::sockets::SharedAssignedSocket;

use super::handler::SyscallHandler;

pub struct UserspaceArgument<T: RefToPointer<T>> {
    inner: T::Out,
}

impl<T: RefToPointer<T>> Constructable<T> for UserspaceArgument<T> {
    fn new(inner: T) -> Self {
        // References are invalid before we did the ptr translation
        // Therefore, replace &T with *const T and &mut T with *mut T
        UserspaceArgument {
            inner: inner.to_pointer_if_ref(),
        }
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

/// I know this is really unreadable. However, I like to learn the power of
/// the type system and traits.
/// What this impl does is basically implement validation for all RefToPointer
/// where Out is an FatPointer.
impl<Ptr: Pointer, T: RefToPointer<T, Out = FatPointer<Ptr>>> Validatable<T>
    for UserspaceArgument<T>
{
    type Error = ValidationError;

    fn validate(self, handler: &mut SyscallHandler) -> Result<T, Self::Error> {
        let start = self.inner.ptr();
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
            unsafe { Ok(T::to_ref_if_pointer(FatPointer::new(ptr, len))) }
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
