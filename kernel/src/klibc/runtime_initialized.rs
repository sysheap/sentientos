use core::{cell::UnsafeCell, mem::MaybeUninit, ops::Deref, sync::atomic::AtomicBool};

pub struct RuntimeInitializedData<T> {
    initialized: AtomicBool,
    data: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Sync for RuntimeInitializedData<T> {}

impl<T> RuntimeInitializedData<T> {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            data: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn initialize(&self, value: T) {
        if self
            .initialized
            .swap(true, core::sync::atomic::Ordering::SeqCst)
        {
            panic!("RuntimeInitializedData already initialized");
        }
        unsafe {
            self.data.get().write(MaybeUninit::new(value));
        }
    }
}

impl<T> Deref for RuntimeInitializedData<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        assert!(
            self.initialized.load(core::sync::atomic::Ordering::SeqCst),
            "RuntimeInitializedData not initialized",
        );
        unsafe { (*self.data.get()).assume_init_ref() }
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeInitializedData;

    #[test_case]
    fn check_initialized_value() {
        let runtime_init = RuntimeInitializedData::<u8>::new();
        assert!(
            !runtime_init
                .initialized
                .load(core::sync::atomic::Ordering::SeqCst)
        );
        runtime_init.initialize(42);
        assert!(
            runtime_init
                .initialized
                .load(core::sync::atomic::Ordering::SeqCst)
        );
    }

    #[test_case]
    fn check_return_value() {
        let runtime_init = RuntimeInitializedData::<u8>::new();
        runtime_init.initialize(42);
        assert_eq!(*runtime_init, 42);
    }
}
