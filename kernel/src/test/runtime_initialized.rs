#[cfg(test)]
mod tests {
    use common::runtime_initialized::RuntimeInitializedData;

    #[test_case]
    fn check_initialized_value() {
        let runtime_init = RuntimeInitializedData::<u8>::new();
        assert!(
            !runtime_init
                .initialized()
                .load(core::sync::atomic::Ordering::SeqCst)
        );
        runtime_init.initialize(42);
        assert!(
            runtime_init
                .initialized()
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
