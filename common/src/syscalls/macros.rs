macro_rules! syscalls {
    ($($name:ident$(<$lt:lifetime>)?($($arg_name:ident: $arg_ty:ty),*) -> $ret:ty);* $(;)?) => {
        $(
            #[allow(non_camel_case_types)]
            #[derive(Debug)]
            struct ${concat($name, Argument)}$(<$lt>)? {
                $(
                    pub $arg_name: $arg_ty,
                )*
            }

            pub fn $name$(<$lt>)?($($arg_name: $arg_ty),*) -> $ret {
                let mut arguments = ${concat($name, Argument)} {
                  $($arg_name,)*
                };
                let mut ret = core::mem::MaybeUninit::<$ret>::uninit();
                let successful: usize;
                unsafe {
                    core::arch::asm!(
                        "ecall",
                        in("a0") ${index()},
                        in("a1") &mut arguments,
                        in("a2") &mut ret,
                        lateout("a0") successful,
                    );
                }
                let status = $crate::syscalls::SyscallStatus::try_from(successful);

                if status != Ok($crate::syscalls::SyscallStatus::Success) {
                    panic!("Could not execute syscall {}: {:?}", stringify!($name), status);
                }
                unsafe {
                    ret.assume_init()
                }
            }
        )*


        pub mod kernel {
            use super::*;
            use $crate::constructable::Constructable;
            use $crate::ref_conversion::RefToPointer;

            pub trait KernelSyscalls {

                type ArgWrapper<T: RefToPointer<T>>: $crate::constructable::Constructable<T>;

                // Syscall functions
                $(fn $name$(<$lt>)?(&mut self, $($arg_name: Self::ArgWrapper<$arg_ty>),*) -> $ret;)*

                /// Validate a pointer such that it is a valid userspace pointer
                fn validate_and_translate_pointer<T, PTR: $crate::pointer::Pointer<T>>(&self, ptr: PTR) -> Option<PTR>;

                fn dispatch(&mut self, nr: usize, arg: usize, ret: usize) -> $crate::syscalls::SyscallStatus {
                    use $crate::syscalls::SyscallStatus;
                    match nr {
                        $(${index()} => {
                            let arg_ptr = $crate::unwrap_or_return!(self.validate_and_translate_pointer(arg as *mut ${concat($name, Argument)}), SyscallStatus::InvalidArgPtr);
                            let ret_ptr = $crate::unwrap_or_return!(self.validate_and_translate_pointer(ret as *mut core::mem::MaybeUninit::<$ret>), SyscallStatus::InvalidRetPtr);
                            // SAFETY: We just validated the pointers
                            let (arg_ref, ret_ref) = unsafe {
                                (&mut *arg_ptr, &mut *ret_ptr)
                            };
                            ret_ref.write(self.$name($(Self::ArgWrapper::new(arg_ref.$arg_name)),*));
                            $crate::syscalls::SyscallStatus::Success
                        })*
                        _ => $crate::syscalls::SyscallStatus::InvalidSyscallNumber
                    }
                }
            }
        }
    };
}

pub(super) use syscalls;
