macro_rules! linux_syscalls {
    ($($number:ident => $name:ident ($($arg_name: ident: $arg_ty:ty),*);)*) => {
        use $crate::syscalls::linux_validator::LinuxUserspaceArg;
        pub trait LinuxSyscalls {
            $(fn $name(&mut self, $($arg_name: LinuxUserspaceArg<$arg_ty>),*) -> isize;)*

            fn handle(&mut self, trap_frame: &TrapFrame) -> isize {
                let nr = trap_frame[Register::a7];
                let args = [trap_frame[Register::a0], trap_frame[Register::a1], trap_frame[Register::a2]];
                match nr {
                    $(headers::syscalls::$number => self.$name($(LinuxUserspaceArg::<$arg_ty>::new(args[${index()}])),*)),*,
                    syscall_nr => {
                        let name = headers::syscalls::SYSCALL_NAMES
                            .iter()
                            .find_map(|(nr, name)| if *nr == syscall_nr { Some(*name) } else { None })
                            .unwrap_or("");
                        panic!("Syscall {name} {syscall_nr} not implemented");
                    }
                }
            }
        }
    };
}

pub(super) use linux_syscalls;
