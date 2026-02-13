use core::{
    cell::UnsafeCell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[cfg(not(test))]
use crate::cpu::Cpu;

const NO_OWNER: usize = usize::MAX;

#[derive(Debug)]
pub struct Spinlock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
    // We can manually disarm the spinlock to not check for locks
    // in the future. This is highly unsafe and only useful to
    // unlock the uart spinlock in case of a panic.
    disarmed: AtomicBool,
    owner_cpu: AtomicUsize,
}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            disarmed: AtomicBool::new(false),
            owner_cpu: AtomicUsize::new(NO_OWNER),
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
            self.set_owner();
            let lock = SpinlockGuard { spinlock: self };
            return Some(f(lock));
        }
        None
    }

    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        if self.disarmed.load(Ordering::SeqCst) {
            return SpinlockGuard { spinlock: self };
        }
        self.detect_same_cpu_deadlock();
        let mut spin_count: u32 = 0;
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_count += 1;
            self.warn_possible_deadlock(spin_count);
            core::hint::spin_loop();
        }
        self.set_owner();
        SpinlockGuard { spinlock: self }
    }

    #[cfg(not(test))]
    fn detect_same_cpu_deadlock(&self) {
        if self.locked.load(Ordering::Relaxed) {
            let cpu_id = Cpu::cpu_id();
            assert_ne!(
                self.owner_cpu.load(Ordering::Relaxed),
                cpu_id,
                "Spinlock deadlock: CPU {cpu_id} tried to re-acquire a lock it already holds"
            );
        }
    }

    #[cfg(test)]
    fn detect_same_cpu_deadlock(&self) {}

    #[cfg(not(test))]
    fn warn_possible_deadlock(&self, spin_count: u32) {
        if spin_count.is_multiple_of(10_000_000) {
            let cpu_id = Cpu::cpu_id();
            let owner = self.owner_cpu.load(Ordering::Relaxed);
            crate::warn!(
                "Spinlock likely deadlocked: CPU {} waiting for lock held by CPU {} ({} spins)",
                cpu_id,
                owner,
                spin_count
            );
        }
    }

    #[cfg(test)]
    fn warn_possible_deadlock(&self, _spin_count: u32) {}

    #[cfg(not(test))]
    fn set_owner(&self) {
        self.owner_cpu.store(Cpu::cpu_id(), Ordering::Relaxed);
    }

    #[cfg(test)]
    fn set_owner(&self) {}

    fn clear_owner(&self) {
        self.owner_cpu.store(NO_OWNER, Ordering::Relaxed);
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
        self.spinlock.clear_owner();
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

    use super::{NO_OWNER, Spinlock};
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

    #[test_case]
    fn owner_cpu_cleared_after_unlock() {
        let spinlock = Spinlock::new(42);
        assert_eq!(spinlock.owner_cpu.load(Ordering::Relaxed), NO_OWNER);
        {
            let _lock = spinlock.lock();
        }
        assert_eq!(spinlock.owner_cpu.load(Ordering::Relaxed), NO_OWNER);
    }

    #[test_case]
    fn try_with_lock_clears_owner() {
        let spinlock = Spinlock::new(42);
        spinlock.try_with_lock(|_| {});
        assert_eq!(spinlock.owner_cpu.load(Ordering::Relaxed), NO_OWNER);
    }
}
