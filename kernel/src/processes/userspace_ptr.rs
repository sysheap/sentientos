use common::{mutex::MutexGuard, pointer::Pointer};
use headers::errno::Errno;

use crate::processes::process::Process;

// SAFETY: Userspace pointer can safely moved between Kernel threads.
unsafe impl<PTR: Pointer> Send for UserspacePtr<PTR> {}

#[derive(Debug)]
pub struct UserspacePtr<PTR: Pointer> {
    /// Pointer is a userspace pointer
    ptr: PTR,
}

impl<PTR: Pointer> UserspacePtr<PTR> {
    pub fn new(ptr: PTR) -> Self {
        Self { ptr }
    }

    pub unsafe fn get(&self) -> PTR {
        self.ptr
    }
}

impl<T> UserspacePtr<*mut T> {
    pub fn write_with_process_lock(
        &self,
        process_lock: &MutexGuard<'_, Process>,
        value: T,
    ) -> Result<(), Errno> {
        process_lock.write_userspace_ptr(self, value)
    }
}
