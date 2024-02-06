pub trait SyscallArgument {
    fn into_reg(self) -> usize;
    fn from_reg(value: usize) -> Self;
}

impl SyscallArgument for char {
    fn into_reg(self) -> usize {
        self as usize
    }

    fn from_reg(value: usize) -> Self {
        value as u8 as char
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

    fn from_reg(value: usize) -> Self {
        ()
    }
}

impl<T> SyscallArgument for *mut T {
    fn into_reg(self) -> usize {
        self as usize
    }

    fn from_reg(value: usize) -> Self {
        value as *mut T
    }
}
