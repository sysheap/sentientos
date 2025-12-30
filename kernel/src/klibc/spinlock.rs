use core::{
    cell::UnsafeCell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug)]
pub struct Spinlock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
    // We can manually disarm the spinlock to not check for locks
    // in the future. This is highly unsafe and only useful to
    // unlock the uart spinlock in case of a panic.
    disarmed: AtomicBool,
}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            disarmed: AtomicBool::new(false),
        }
    }

    pub fn with_lock<'a, R>(&'a self, f: impl FnOnce(SpinlockGuard<'a, T>) -> R) -> R {
        let lock = self.lock();
        f(lock)
    }

    pub fn try_with_lock<'a, R>(&'a self, f: impl FnOnce(SpinlockGuard<'a, T>) -> R) -> Option<R> {
        let value = self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed);
        if value.is_ok() {
            let lock = SpinlockGuard { spinlock: self };
            return Some(f(lock));
        }
        None
    }

    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        if self.disarmed.load(Ordering::SeqCst) {
            return SpinlockGuard { spinlock: self };
        }
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinlockGuard { spinlock: self }
    }

    #[cfg(test)]
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }

    /// # Safety
    /// This is actual never save and should only be used
    /// in very space places (like stdout protection)
    pub unsafe fn disarm(&self) {
        self.disarmed.store(true, Ordering::SeqCst);
    }
}

unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}

pub struct SpinlockGuard<'a, T> {
    spinlock: &'a Spinlock<T>,
}

impl<T> Drop for SpinlockGuard<'_, T> {
    fn drop(&mut self) {
        self.spinlock.locked.store(false, Ordering::Release);
    }
}

impl<T> Deref for SpinlockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We're (the SpinlockGuard) have exclusive rights to the data
        unsafe { &*self.spinlock.data.get() }
    }
}

impl<T> DerefMut for SpinlockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: We're (the SpinlockGuard) have exclusive rights to the data
        unsafe { &mut *self.spinlock.data.get() }
    }
}

impl<T: Debug> Debug for SpinlockGuard<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // SAFETY: We're (the SpinlockGuard) have exclusive rights to the data
        unsafe { writeln!(f, "SpinlockGuard {{\n{:?}\n}}", *self.spinlock.data.get()) }
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;

    use super::Spinlock;
    use crate::debug;

    #[test_case]
    fn with_lock() {
        let spinlock = Spinlock::new(42);
        assert!(!spinlock.locked.load(Ordering::Acquire));
        let result = spinlock.with_lock(|mut d| {
            *d = 45;
            *d
        });
        assert!(!spinlock.locked.load(Ordering::Acquire));
        unsafe {
            assert_eq!(*spinlock.data.get(), 45);
        }
        assert_eq!(result, 45);
    }

    #[test_case]
    fn check_lock_and_unlock() {
        let spinlock = Spinlock::new(42);
        assert!(!spinlock.locked.load(Ordering::Acquire));
        {
            let mut locked = spinlock.lock();
            assert!(spinlock.locked.load(Ordering::Acquire));
            *locked = 1;
        }
        assert!(!spinlock.locked.load(Ordering::Acquire));
        unsafe {
            assert_eq!(*spinlock.data.get(), 1);
        }
        let mut locked = spinlock.lock();
        *locked = 42;
        assert!(spinlock.locked.load(Ordering::Acquire));
        unsafe {
            assert_eq!(*spinlock.data.get(), 42);
        }
    }

    #[test_case]
    fn check_disarm() {
        let spinlock = Spinlock::new(42);
        let _lock = spinlock.lock();
        unsafe {
            spinlock.disarm();
        }
        let _lock2 = spinlock.lock();
    }

    #[test_case]
    fn print_doesnt_deadlock() {
        let spinlock = Spinlock::new(42);
        debug!("{spinlock:?}");
        let spinlock_guard = spinlock.lock();
        debug!("{spinlock_guard:?}");
    }
}
