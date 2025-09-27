use common::mutex::MutexGuard;
use headers::errno::Errno;

use crate::processes::process::{Process, ProcessWeakRef};

// SAFETY: Userspace pointer can safely moved between Kernel threads.
unsafe impl<T> Send for UserspacePtrMut<T> {}

#[derive(Debug)]
pub struct UserspacePtrMut<T> {
    /// Pointer is a userspace pointer
    ptr: *mut T,
    _process: ProcessWeakRef,
}

impl<T> UserspacePtrMut<T> {
    pub fn new(ptr: *mut T, process: ProcessWeakRef) -> Self {
        Self {
            ptr,
            _process: process,
        }
    }

    pub fn write_with_process_lock(
        &self,
        process_lock: &MutexGuard<'_, Process>,
        value: T,
    ) -> Result<(), Errno> {
        process_lock.write_userspace_ptr(self.ptr, value)
    }
}
