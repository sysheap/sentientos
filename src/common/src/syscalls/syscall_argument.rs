use super::{SysExecuteError, SysWaitError};

pub trait SyscallArgument {
    fn into_reg(self) -> usize;
    fn from_reg(value: usize) -> Self;
}

pub trait SyscallReturnArgument {
    fn into_double_reg(self) -> (usize, usize);
    fn from_double_reg(first: usize, second: usize) -> Self;
}

impl<T: SyscallArgument> SyscallReturnArgument for T {
    fn into_double_reg(self) -> (usize, usize) {
        (self.into_reg(), 0)
    }

    fn from_double_reg(first: usize, second: usize) -> Self {
        T::from_reg(first)
    }
}

impl<T: SyscallArgument, E: SyscallArgument> SyscallReturnArgument for Result<T, E> {
    fn into_double_reg(self) -> (usize, usize) {
        match self {
            Ok(value) => (0, value.into_reg()),
            Err(error) => (1, error.into_reg()),
        }
    }

    fn from_double_reg(first: usize, second: usize) -> Self {
        if first == 0 {
            Ok(T::from_reg(second))
        } else {
            Err(E::from_reg(second))
        }
    }
}

impl<T: SyscallArgument> SyscallReturnArgument for Option<T> {
    fn into_double_reg(self) -> (usize, usize) {
        match self {
            Some(value) => (0, value.into_reg()),
            None => (1, 0),
        }
    }

    fn from_double_reg(first: usize, second: usize) -> Self {
        if first == 0 {
            Some(T::from_reg(second))
        } else {
            None
        }
    }
}

impl SyscallArgument for char {
    fn into_reg(self) -> usize {
        self as usize
    }

    fn from_reg(value: usize) -> Self {
        value as u8 as char
    }
}

impl SyscallArgument for u8 {
    fn into_reg(self) -> usize {
        self as usize
    }

    fn from_reg(value: usize) -> Self {
        value as u8
    }
}

impl SyscallArgument for isize {
    fn into_reg(self) -> usize {
        self as usize
    }

    fn from_reg(value: usize) -> Self {
        value as isize
    }
}

impl SyscallArgument for usize {
    fn into_reg(self) -> usize {
        self
    }

    fn from_reg(value: usize) -> Self {
        value
    }
}

impl SyscallArgument for u64 {
    fn into_reg(self) -> usize {
        self as usize
    }

    fn from_reg(value: usize) -> Self {
        value as u64
    }
}

impl<T> SyscallArgument for &T {
    fn into_reg(self) -> usize {
        self as *const T as usize
    }

    fn from_reg(value: usize) -> Self {
        unsafe { &*(value as *const T) }
    }
}

impl SyscallArgument for () {
    fn into_reg(self) -> usize {
        0
    }

    fn from_reg(value: usize) -> Self {}
}

impl<T> SyscallArgument for *mut T {
    fn into_reg(self) -> usize {
        self as usize
    }

    fn from_reg(value: usize) -> Self {
        value as *mut T
    }
}

impl SyscallArgument for SysWaitError {
    fn into_reg(self) -> usize {
        self as usize
    }

    fn from_reg(value: usize) -> Self {
        unsafe { core::mem::transmute(value) }
    }
}

impl SyscallArgument for SysExecuteError {
    fn into_reg(self) -> usize {
        self as usize
    }

    fn from_reg(value: usize) -> Self {
        unsafe { core::mem::transmute(value) }
    }
}
