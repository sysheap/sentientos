use core::{
    cell::UnsafeCell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug)]
pub struct Mutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
    // We can manually disarm the mutex to not check for locks
    // in the future. This is highly unsafe and only useful to
    // unlock the uart mutex in case of a panic.
    disarmed: AtomicBool,
}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            disarmed: AtomicBool::new(false),
        }
    }

    pub fn with_lock<'a, R>(&'a self, f: impl FnOnce(MutexGuard<'a, T>) -> R) -> R {
        let lock = self.lock();
        f(lock)
    }

    pub fn lock(&self) -> MutexGuard<T> {
        if self.disarmed.load(Ordering::SeqCst) {
            return MutexGuard { mutex: self };
        }
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            panic!("Lock held twice.");
            // core::hint::spin_loop();
        }
        MutexGuard { mutex: self }
    }

    #[doc(hidden)]
    pub fn get_locked(&self) -> &AtomicBool {
        &self.locked
    }

    #[doc(hidden)]
    pub fn get_data(&self) -> &UnsafeCell<T> {
        &self.data
    }

    /// # Safety
    /// This is actual never save and should only be used
    /// in very space places (like stdout protection)
    pub unsafe fn disarm(&self) {
        self.disarmed.store(true, Ordering::SeqCst);
    }
}

unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We're (the MutexGuard) have exlusive rights to the data
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: We're (the MutexGuard) have exlusive rights to the data
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T: Debug> Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // SAFETY: We're (the MutexGuard) have exlusive rights to the data
        unsafe { writeln!(f, "MutexGuard {{\n{:?}\n}}", *self.mutex.data.get()) }
    }
}
