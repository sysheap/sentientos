use core::{ffi::c_void, marker::PhantomData};

pub struct LinuxUserspaceArg<T> {
    arg: usize,
    phantom: PhantomData<T>,
}

impl<T> LinuxUserspaceArg<T> {
    pub fn new(arg: usize) -> Self {
        Self {
            arg,
            phantom: PhantomData,
        }
    }
}

macro_rules! simple_validate {
    ($ty:ty) => {
        impl LinuxUserspaceArg<$ty> {
            pub fn validate(self) -> $ty {
                self.arg as $ty
            }
        }
    };
}

simple_validate!(i32);
simple_validate!(usize);

impl LinuxUserspaceArg<*const c_void> {
    pub fn validate(self) -> *const c_void {
        self.arg as *const c_void
    }
}
