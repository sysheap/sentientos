use core::marker::PhantomData;

use common::{mutex::MutexGuard, unwrap_or_return};
use headers::errno::Errno;

use crate::processes::{
    process::{Process, ProcessRef},
    userspace_ptr::UserspacePtrMut,
};

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

macro_rules! simple_validate {
    ($ty:ty) => {
        impl LinuxUserspaceArg<$ty> {
            pub fn validate(self) -> $ty {
                self.arg as $ty
            }
        }
    };
}

simple_validate!(i32);
simple_validate!(usize);

/// Contains a ref into the processes address space, therefore keeps a MutexGuard
/// to the process so the memory mapping cannot be changed.
pub struct ProcessRefGuard<'a, T: ?Sized> {
    reference: &'a T,
    // Field is never used, it is only there to keep the process locked
    _process: MutexGuard<'a, Process>,
}

impl<'a, T: ?Sized> ProcessRefGuard<'a, T> {
    pub fn get(&self) -> &'a T {
        self.reference
    }
}

impl LinuxUserspaceArg<*const u8> {
    pub fn validate_str<'a>(&'a self, len: usize) -> Result<ProcessRefGuard<'a, str>, Errno> {
        let process_guard = self.process.lock();
        let ptr = self.process.with_lock(|p| {
            let pt = p.get_page_table();
            let ptr = self.arg as *const u8;
            if !pt.is_valid_userspace_fat_ptr(ptr, len, false) {
                return None;
            }
            pt.translate_userspace_address_to_physical_address(ptr)
        });
        let ptr = unwrap_or_return!(ptr, Err(Errno::EFAULT));
        let slice = unsafe { core::str::from_raw_parts(ptr, len) };
        Ok(ProcessRefGuard {
            reference: slice,
            _process: process_guard,
        })
    }
}

impl<T> LinuxUserspaceArg<*mut T> {
    pub fn as_userspace_ptr(&self) -> UserspacePtrMut<T> {
        UserspacePtrMut::new(self.arg as *mut T, ProcessRef::downgrade(&self.process))
    }
}
