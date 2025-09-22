use core::ffi::{c_int, c_uint};

use crate::{print, processes::process::ProcessRef, syscalls::macros::linux_syscalls};
use common::{
    constructable::Constructable,
    syscalls::{
        kernel::KernelSyscalls,
        trap_frame::{Register, TrapFrame},
    },
};

use crate::syscalls::{handler::SyscallHandler, validator::UserspaceArgument};
use headers::{
    errno::EBADF,
    syscall_types::{pollfd, sigaction, sigset_t, stack_t, timespec},
};

linux_syscalls! {
    SYSCALL_NR_EXIT_GROUP => exit_group(status: c_int);
    SYSCALL_NR_PPOLL => ppoll(fds: *mut pollfd, n: c_uint, to: *const timespec, mask: *const sigset_t);
    SYSCALL_NR_RT_SIGACTION => rt_sigaction(sig: c_int, act: *const sigaction, oact: *mut sigaction, sigsetsize: usize);
    SYSCALL_NR_RT_SIGPROCMASK => rt_sigprocmask(how: c_int, set: *const sigset_t, oldset: *mut sigset_t, sigsetsize: usize);
    SYSCALL_NR_SET_TID_ADDRESS => set_tid_address(tidptr: *mut c_int);
    SYSCALL_NR_SIGALTSTACK => sigaltstack(uss: *const stack_t, uoss: *mut stack_t);
    SYSCALL_NR_WRITE => write(fd: c_int, buf: *const u8, count: usize);
}

pub struct LinuxSyscallHandler {
    handler: SyscallHandler,
}

impl LinuxSyscalls for LinuxSyscallHandler {
    fn write(
        &mut self,
        fd: LinuxUserspaceArg<i32>,
        buf: LinuxUserspaceArg<*const u8>,
        count: LinuxUserspaceArg<usize>,
    ) -> isize {
        let fd: i32 = fd.validate();
        if fd != 1 && fd != 2 {
            return -EBADF;
        }

        if fd == 2 {
            print!("ERROR: ");
        }

        let count = count.validate();
        let buf = match buf.validate_str(count) {
            Ok(guard) => guard,
            Err(err) => return err,
        };

        print!("{}", buf.get());

        count as isize
    }

    fn exit_group(&mut self, status: LinuxUserspaceArg<c_int>) -> isize {
        let status = status.validate();
        self.handler
            .sys_exit(UserspaceArgument::new(status as isize));
        0
    }

    fn set_tid_address(&mut self, _tidptr: LinuxUserspaceArg<*mut c_int>) -> isize {
        0
    }

    fn ppoll(
        &mut self,
        _fds: LinuxUserspaceArg<*mut pollfd>,
        _n: LinuxUserspaceArg<c_uint>,
        _to: LinuxUserspaceArg<*const timespec>,
        _mask: LinuxUserspaceArg<*const sigset_t>,
    ) -> isize {
        0
    }

    fn rt_sigaction(
        &mut self,
        _sig: LinuxUserspaceArg<c_int>,
        _act: LinuxUserspaceArg<*const sigaction>,
        _oact: LinuxUserspaceArg<*mut sigaction>,
        _sigsetsize: LinuxUserspaceArg<usize>,
    ) -> isize {
        0
    }

    fn rt_sigprocmask(
        &mut self,
        _how: LinuxUserspaceArg<c_int>,
        _set: LinuxUserspaceArg<*const sigset_t>,
        _oldset: LinuxUserspaceArg<*mut sigset_t>,
        _sigsetsize: LinuxUserspaceArg<usize>,
    ) -> isize {
        0
    }

    fn sigaltstack(
        &mut self,
        _uss: LinuxUserspaceArg<*const stack_t>,
        _uoss: LinuxUserspaceArg<*mut stack_t>,
    ) -> isize {
        0
    }

    fn get_process(&self) -> ProcessRef {
        self.handler.current_process().clone()
    }
}

impl LinuxSyscallHandler {
    pub fn new() -> Self {
        Self {
            handler: SyscallHandler::new(),
        }
    }
}
