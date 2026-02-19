use core::any::Any;

use crate::{numbers::Number, pid::Tid, pointer::FatPointer};
use alloc::{boxed::Box, vec::Vec};

extern crate alloc;

/// This type will be used to store temporary data via the syscall
/// We need that to pass nested types to the kernel (&[&str]) for example.
/// Because we need to translate the inner types to FatPointer we have to
/// allocate new memory. We could also do dirty tricks by relying on the
/// representation of &str in memory but rather not do that. The compiler
/// could change it any time.
#[derive(Default)]
pub struct SyscallTempStorage {
    vecs: Vec<Box<dyn Any>>,
}

impl SyscallTempStorage {
    pub fn add<T: 'static>(&mut self, vec: Vec<T>) {
        self.vecs.push(Box::new(vec));
    }
}

pub trait SyscallArgument {
    type Converted: Copy + Clone;

    fn convert(self, storage: &mut SyscallTempStorage) -> Self::Converted;
}

impl<T: Number> SyscallArgument for T {
    type Converted = T;

    fn convert(self, _storage: &mut SyscallTempStorage) -> Self::Converted {
        self
    }
}

impl SyscallArgument for char {
    type Converted = char;

    fn convert(self, _storage: &mut SyscallTempStorage) -> Self::Converted {
        self
    }
}

impl SyscallArgument for &str {
    type Converted = FatPointer<*const u8>;

    fn convert(self, _storage: &mut SyscallTempStorage) -> Self::Converted {
        FatPointer::new(self.as_ptr(), self.len())
    }
}

impl<T: Number> SyscallArgument for &[T] {
    type Converted = FatPointer<*const T>;

    fn convert(self, _storage: &mut SyscallTempStorage) -> Self::Converted {
        FatPointer::new(self.as_ptr(), self.len())
    }
}

impl<T: Number> SyscallArgument for &mut [T] {
    type Converted = FatPointer<*mut T>;

    fn convert(self, _storage: &mut SyscallTempStorage) -> Self::Converted {
        FatPointer::new(self.as_mut_ptr(), self.len())
    }
}

impl<'a> SyscallArgument for &'a [&'a str] {
    type Converted = FatPointer<*const FatPointer<*const u8>>;

    fn convert(self, storage: &mut SyscallTempStorage) -> Self::Converted {
        let temp_vec: Vec<FatPointer<*const u8>> = self
            .iter()
            .map(|s| FatPointer::new(s.as_ptr(), s.len()))
            .collect();
        let converted = FatPointer::new(temp_vec.as_ptr(), temp_vec.len());

        storage.add(temp_vec);

        converted
    }
}

impl SyscallArgument for Tid {
    type Converted = Tid;

    fn convert(self, storage: &mut SyscallTempStorage) -> Self::Converted {
        self
    }
}
