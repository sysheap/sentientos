use crate::{net::UDPDescriptor, numbers::Number, pointer::FatPointer};

pub trait SyscallArgument {
    type Converted: Clone + Copy;

    fn convert(self) -> Self::Converted;
}

impl<T: Number> SyscallArgument for T {
    type Converted = T;

    fn convert(self) -> Self::Converted {
        self
    }
}

impl SyscallArgument for char {
    type Converted = char;

    fn convert(self) -> Self::Converted {
        self
    }
}

impl SyscallArgument for &str {
    type Converted = FatPointer<*const u8>;

    fn convert(self) -> Self::Converted {
        FatPointer::new(self.as_ptr(), self.len())
    }
}

impl<T: Number> SyscallArgument for &[T] {
    type Converted = FatPointer<*const T>;

    fn convert(self) -> Self::Converted {
        FatPointer::new(self.as_ptr(), self.len())
    }
}

impl<T: Number> SyscallArgument for &mut [T] {
    type Converted = FatPointer<*mut T>;

    fn convert(self) -> Self::Converted {
        FatPointer::new(self.as_mut_ptr(), self.len())
    }
}

impl SyscallArgument for UDPDescriptor {
    type Converted = UDPDescriptor;

    fn convert(self) -> Self::Converted {
        self
    }
}
