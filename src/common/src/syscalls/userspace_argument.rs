pub struct UserspaceArgument<T> {
    value: T,
}

impl<T> UserspaceArgument<T> {
    pub fn new(value: T) -> Self {
        UserspaceArgument { value }
    }
}

/// WARNING: Using this trait is dangerous because it allows to bypass the validation of the userspace argument
pub trait UserspaceArgumentValueExtractor<T> {
    fn get(self) -> T;
}

impl<T> UserspaceArgumentValueExtractor<T> for UserspaceArgument<T> {
    fn get(self) -> T {
        self.value
    }
}
