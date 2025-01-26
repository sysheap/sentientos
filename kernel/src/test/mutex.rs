#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;

    use common::mutex::Mutex;

    use crate::debug;

    #[test_case]
    fn with_lock() {
        let mutex = Mutex::new(42);
        assert!(!mutex.get_locked().load(Ordering::Acquire));
        let result = mutex.with_lock(|mut d| {
            *d = 45;
            *d
        });
        assert!(!mutex.get_locked().load(Ordering::Acquire));
        unsafe {
            assert_eq!(*mutex.get_data().get(), 45);
        }
        assert_eq!(result, 45);
    }

    #[test_case]
    fn check_lock_and_unlock() {
        let mutex = Mutex::new(42);
        assert!(!mutex.get_locked().load(Ordering::Acquire));
        {
            let mut locked = mutex.lock();
            assert!(mutex.get_locked().load(Ordering::Acquire));
            *locked = 1;
        }
        assert!(!mutex.get_locked().load(Ordering::Acquire));
        unsafe {
            assert_eq!(*mutex.get_data().get(), 1);
        }
        let mut locked = mutex.lock();
        *locked = 42;
        assert!(mutex.get_locked().load(Ordering::Acquire));
        unsafe {
            assert_eq!(*mutex.get_data().get(), 42);
        }
    }

    #[test_case]
    fn check_disarm() {
        let mutex = Mutex::new(42);
        let _lock = mutex.lock();
        unsafe {
            mutex.disarm();
        }
        let _lock2 = mutex.lock();
    }

    #[test_case]
    fn print_doesnt_deadlock() {
        let mutex = Mutex::new(42);
        debug!("{mutex:?}");
        let mutex_guard = mutex.lock();
        debug!("{mutex_guard:?}");
    }
}
