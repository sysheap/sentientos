use core::{
    cell::UnsafeCell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Mutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T: Debug> Debug for Mutex<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.lock().fmt(f)
    }
}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        MutexGuard { mutex: self }
    }
}

unsafe impl<T> Sync for Mutex<T> {}
unsafe impl<T> Send for Mutex<T> {}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;

    use crate::klibc::Mutex;

    #[test_case]
    fn check_lock_and_unlock() {
        let mutex = Mutex::new(42);
        assert_eq!(mutex.locked.load(Ordering::Acquire), false);
        {
            let mut locked = mutex.lock();
            assert_eq!(mutex.locked.load(Ordering::Acquire), true);
            *locked = 1;
        }
        assert_eq!(mutex.locked.load(Ordering::Acquire), false);
        unsafe {
            assert_eq!(*mutex.data.get(), 1);
        }
        let mut locked = mutex.lock();
        *locked = 42;
        assert_eq!(mutex.locked.load(Ordering::Acquire), true);
        unsafe {
            assert_eq!(*mutex.data.get(), 42);
        }
    }
}
