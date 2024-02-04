#[macro_export]
macro_rules! ecall {
    ($syscall:expr,) => {
        ecall_0($syscall as usize)
    };
    ($syscall:expr, $arg1:expr) => {
        ecall_1($syscall as usize, $arg1.into_reg())
    };
    ($syscall:expr, $arg1:expr, $arg2:expr) => {
        ecall_2($syscall as usize, $arg1.into_reg(), $arg2.into_reg())
    };
}

#[macro_export]
macro_rules! syscalls {
    ($($name:ident($($arg_name:ident: $arg_ty:ty),*) -> $ret:ty);* $(;)?) => {
        #[repr(usize)]
        #[allow(non_camel_case_types)]
        pub enum Syscalls {
            $($name,)*
        }

        $(
            pub fn $name($($arg_name: $arg_ty),*) -> $ret {
                <$ret>::from_reg(ecall!(Syscalls::$name, $($arg_name),*))
            }
        )*


        pub mod kernel {
            use super::UserspaceArgument;
            use super::syscall_argument::SyscallArgument;

            pub trait KernelSyscalls {
                $(fn $name($($arg_name: UserspaceArgument<$arg_ty>),*) -> $ret;)*
                fn dispatch(nr: usize, arg0: usize, arg1: usize) -> usize {
                    use super::Syscalls;
                    macro_rules! kernel_dispatch_call {
                        ($x:ident,) => { Self::$x().into_reg() };
                        ($x:ident, $arg1:ty) => { Self::$x(UserspaceArgument::new(<$arg1>::from_reg(arg0))).into_reg() };
                        ($x:ident, $arg1:ty, $arg2:ty) => { Self::$x(UserspaceArgument::new(<$arg1>::from_reg(arg0)), UserspaceArgument::new(<$arg2>::from_reg(arg1))).into_reg() };
                    }
                    let enum_value: Syscalls = unsafe { core::mem::transmute(nr) };
                    match enum_value {
                        $(
                            Syscalls::$name => { kernel_dispatch_call!($name, $($arg_ty),*) },
                        )*
                    }
                }
            }
        }
    };
}
