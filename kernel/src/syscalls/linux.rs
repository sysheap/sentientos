use core::ffi::c_int;

use crate::{print, syscalls::macros::linux_syscalls};
use common::{
    constructable::Constructable,
    pointer::FatPointer,
    syscalls::{
        kernel::KernelSyscalls,
        trap_frame::{Register, TrapFrame},
    },
};

use crate::syscalls::{handler::SyscallHandler, validator::UserspaceArgument};

linux_syscalls! {
    SYSCALL_NR_WRITE => write(fd: i32, buf: *const u8, len: usize);
    SYSCALL_NR_EXIT_GROUP => exit_group(status: isize);
    SYSCALL_NR_SET_TID_ADDRESS => set_tid_address(arg1: *const c_int);
    SYSCALL_NR_PPOLL => ppoll_time32();
    SYSCALL_NR_RT_SIGACTION => rt_sigaction();
    SYSCALL_NR_SIGALTSTACK => sigaltstack();
    SYSCALL_NR_RT_SIGPROCMASK => rt_sigprocmask();
}

pub struct LinuxSyscallHandler {
    handler: SyscallHandler,
}

impl LinuxSyscalls for LinuxSyscallHandler {
    fn write(
        &mut self,
        fd: LinuxUserspaceArg<i32>,
        buf: LinuxUserspaceArg<*const u8>,
        len: LinuxUserspaceArg<usize>,
    ) -> isize {
        let fd: i32 = fd.into();
        let buf = buf.into();
        let len = len.into();
        if fd != 1 && fd != 2 {
            return -1;
        }

        if fd == 2 {
            print!("ERROR: ");
        }

        let result = self
            .handler
            .sys_write(UserspaceArgument::new(FatPointer::new(buf, len)));

        if result.is_ok() { len as isize } else { -1 }
    }

    fn exit_group(&mut self, status: LinuxUserspaceArg<isize>) -> isize {
        let status: isize = status.into();
        self.handler.sys_exit(UserspaceArgument::new(status));
        0
    }

    fn set_tid_address(&mut self, _arg1: LinuxUserspaceArg<*const c_int>) -> isize {
        0
    }

    fn ppoll_time32(&mut self) -> isize {
        0
    }

    fn rt_sigaction(&mut self) -> isize {
        0
    }

    fn sigaltstack(&mut self) -> isize {
        0
    }

    fn rt_sigprocmask(&mut self) -> isize {
        0
    }

    // fn tkill(&mut self) -> isize {
    //     0
    // }
}

impl LinuxSyscallHandler {
    pub fn new() -> Self {
        Self {
            handler: SyscallHandler::new(),
        }
    }
}
