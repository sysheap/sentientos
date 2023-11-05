#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;

    use common::mutex::Mutex;

    #[test_case]
    fn check_lock_and_unlock() {
        let mutex = Mutex::new(42);
        assert_eq!(mutex.get_locked().load(Ordering::Acquire), false);
        {
            let mut locked = mutex.lock();
            assert_eq!(mutex.get_locked().load(Ordering::Acquire), true);
            *locked = 1;
        }
        assert_eq!(mutex.get_locked().load(Ordering::Acquire), false);
        unsafe {
            assert_eq!(*mutex.get_data().get(), 1);
        }
        let mut locked = mutex.lock();
        *locked = 42;
        assert_eq!(mutex.get_locked().load(Ordering::Acquire), true);
        unsafe {
            assert_eq!(*mutex.get_data().get(), 42);
        }
    }
}
