pub trait NeedsUserSpaceWrapper {
    type Wrapped;
    fn wrap_arg(value: usize, process: ProcessRef) -> Self::Wrapped;
}

macro_rules! impl_userspace_arg {
    ($type:ty) => {
        impl<T> NeedsUserSpaceWrapper for $type {
            type Wrapped = LinuxUserspaceArg<$type>;
            fn wrap_arg(value: usize, process: ProcessRef) -> Self::Wrapped {
                LinuxUserspaceArg::new(value, process)
            }
        }
    };
}

macro_rules! impl_plain_arg {
    ($type:ty) => {
        impl NeedsUserSpaceWrapper for $type {
            type Wrapped = $type;
            fn wrap_arg(value: usize, _process: ProcessRef) -> Self::Wrapped {
                value as $type
            }
        }
    };
}

impl_userspace_arg!(*const T);
impl_userspace_arg!(*mut T);
impl_userspace_arg!(Option<*const T>);
impl_userspace_arg!(Option<*mut T>);

impl_plain_arg!(c_int);
impl_plain_arg!(c_uint);
impl_plain_arg!(c_ulong);
impl_plain_arg!(usize);
impl_plain_arg!(isize);

macro_rules! linux_syscalls {
    ($($number:ident => $name:ident ($($arg_name: ident: $arg_ty:ty),*);)*) => {
        use $crate::syscalls::linux_validator::LinuxUserspaceArg;
        pub trait LinuxSyscalls {
            $(async fn $name(&mut self, $($arg_name: <$arg_ty as $crate::syscalls::macros::NeedsUserSpaceWrapper>::Wrapped),*) -> Result<isize, headers::errno::Errno>;)*

            fn get_process(&self) -> $crate::processes::process::ProcessRef;

            async fn handle(&mut self, trap_frame: &TrapFrame) -> Result<isize, headers::errno::Errno> {
                let nr = trap_frame[Register::a7];
                let args = [
                    trap_frame[Register::a0],
                    trap_frame[Register::a1],
                    trap_frame[Register::a2],
                    trap_frame[Register::a3],
                    trap_frame[Register::a4],
                    trap_frame[Register::a5]
                ];
                match nr {
                    $(headers::syscalls::$number => self.$name($(<$arg_ty as $crate::syscalls::macros::NeedsUserSpaceWrapper>::wrap_arg(args[${index()}], self.get_process())),*).await),*,
                    syscall_nr => {
                        let pc = $crate::cpu::Cpu::read_sepc();
                        let name = headers::syscalls::SYSCALL_NAMES
                            .iter()
                            .find_map(|(nr, name)| if *nr == syscall_nr { Some(*name) } else { None })
                            .unwrap_or("");
                        panic!("Syscall {name} {syscall_nr} not implemented (pc={pc:#x})");
                    }
                }
            }
        }
    };
}

use core::ffi::{c_int, c_uint, c_ulong};

pub(super) use linux_syscalls;

use crate::{processes::process::ProcessRef, syscalls::linux_validator::LinuxUserspaceArg};
