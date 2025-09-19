use core::marker::PhantomData;

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

impl From<LinuxUserspaceArg<i32>> for i32 {
    fn from(val: LinuxUserspaceArg<i32>) -> Self {
        val.arg as i32
    }
}
impl From<LinuxUserspaceArg<usize>> for usize {
    fn from(val: LinuxUserspaceArg<usize>) -> Self {
        val.arg
    }
}
impl From<LinuxUserspaceArg<isize>> for isize {
    fn from(val: LinuxUserspaceArg<isize>) -> Self {
        val.arg as isize
    }
}
impl From<LinuxUserspaceArg<*const u8>> for *const u8 {
    fn from(val: LinuxUserspaceArg<*const u8>) -> Self {
        val.arg as *const u8
    }
}
