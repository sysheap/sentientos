use common::syscalls::userspace_argument::{UserspaceArgument, UserspaceArgumentValueExtractor};

pub trait FailibleUserspaceArgumentValidator<T> {
    fn validate(self) -> Result<T, ()>;
}

pub trait UserspaceArgumentValidator<T> {
    fn validate(self) -> T;
}

macro_rules! simple_type {
    ($type:ty) => {
        impl UserspaceArgumentValidator<$type> for UserspaceArgument<$type> {
            fn validate(self) -> $type {
                self.get()
            }
        }
    };
}

simple_type!(char);
simple_type!(usize);
simple_type!(isize);
simple_type!(u64);

impl<'a, T: 'a> FailibleUserspaceArgumentValidator<&'a T> for UserspaceArgument<&'a T> {
    fn validate(self) -> Result<&'a T, ()> {
        // TODO: Validate if pointer is valid (probably don't have a generic wrapper for &T)
        Ok(self.get())
    }
}
