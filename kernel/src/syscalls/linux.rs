use core::ffi::{c_int, c_uint};

use crate::{
    print,
    processes::{process::ProcessRef, thread::ThreadRef, timer},
    syscalls::macros::linux_syscalls,
};
use common::{
    constructable::Constructable,
    syscalls::{
        kernel::KernelSyscalls,
        trap_frame::{Register, TrapFrame},
    },
};

use crate::syscalls::{handler::SyscallHandler, validator::UserspaceArgument};
use headers::{
    errno::Errno,
    syscall_types::{pollfd, sigaction, sigset_t, stack_t, timespec},
};

linux_syscalls! {
    SYSCALL_NR_EXIT_GROUP => exit_group(status: c_int);
    SYSCALL_NR_PPOLL => ppoll(fds: *mut pollfd, n: c_uint, to: Option<*const timespec>, mask: Option<*const sigset_t>);
    SYSCALL_NR_RT_SIGACTION => rt_sigaction(sig: c_int, act: *const sigaction, oact: *mut sigaction, sigsetsize: usize);
    SYSCALL_NR_RT_SIGPROCMASK => rt_sigprocmask(how: c_int, set: *const sigset_t, oldset: *mut sigset_t, sigsetsize: usize);
    SYSCALL_NR_SET_TID_ADDRESS => set_tid_address(tidptr: *mut c_int);
    SYSCALL_NR_SIGALTSTACK => sigaltstack(uss: *const stack_t, uoss: *mut stack_t);
    SYSCALL_NR_WRITE => write(fd: c_int, buf: *const u8, count: usize);
    SYSCALL_NR_NANOSLEEP => nanosleep(duration: *const timespec, rem: Option<*const timespec>);
}

pub struct LinuxSyscallHandler {
    handler: SyscallHandler,
}

impl LinuxSyscalls for LinuxSyscallHandler {
    fn write(
        &mut self,
        fd: i32,
        buf: LinuxUserspaceArg<*const u8>,
        count: usize,
    ) -> Result<isize, Errno> {
        if fd != 1 && fd != 2 {
            return Err(Errno::EBADF);
        }

        let string = buf.validate_str(count)?;

        print!("{}", string);

        Ok(count as isize)
    }

    fn exit_group(&mut self, status: c_int) -> Result<isize, Errno> {
        self.handler
            .sys_exit(UserspaceArgument::new(status as isize));
        Ok(0)
    }

    fn set_tid_address(&mut self, tidptr: LinuxUserspaceArg<*mut c_int>) -> Result<isize, Errno> {
        self.handler.current_thread().with_lock(|mut t| {
            t.set_clear_child_tid((&tidptr).into());
        });
        Ok(0)
    }

    fn ppoll(
        &mut self,
        fds: LinuxUserspaceArg<*mut pollfd>,
        n: c_uint,
        to: LinuxUserspaceArg<Option<*const timespec>>,
        mask: LinuxUserspaceArg<Option<*const sigset_t>>,
    ) -> Result<isize, Errno> {
        let _fds = fds.validate_slice(n as usize)?;
        let _to = to.validate_ptr()?;
        let _mask = mask.validate_ptr()?;
        Ok(0)
    }

    fn rt_sigaction(
        &mut self,
        _sig: c_int,
        _act: LinuxUserspaceArg<*const sigaction>,
        _oact: LinuxUserspaceArg<*mut sigaction>,
        _sigsetsize: usize,
    ) -> Result<isize, Errno> {
        Ok(0)
    }

    fn rt_sigprocmask(
        &mut self,
        _how: c_int,
        _set: LinuxUserspaceArg<*const sigset_t>,
        _oldset: LinuxUserspaceArg<*mut sigset_t>,
        _sigsetsize: usize,
    ) -> Result<isize, Errno> {
        Ok(0)
    }

    fn sigaltstack(
        &mut self,
        uss: LinuxUserspaceArg<*const stack_t>,
        _uoss: LinuxUserspaceArg<*mut stack_t>,
    ) -> Result<isize, Errno> {
        let _uss = uss.validate_ptr()?;
        Ok(0)
    }

    fn get_process(&self) -> ProcessRef {
        self.handler.current_process().clone()
    }

    fn nanosleep(
        &mut self,
        duration: LinuxUserspaceArg<*const timespec>,
        _rem: LinuxUserspaceArg<Option<*const timespec>>,
    ) -> Result<isize, Errno> {
        let duration = duration.validate_ptr()?;
        if duration.tv_sec < 0 || !(0..999999999).contains(&duration.tv_nsec) {
            return Err(Errno::EINVAL);
        }
        self.handler.current_thread().with_lock(|mut t| {
            t.set_waiting_on_syscall::<Result<isize, Errno>>();
        });
        timer::register_wakeup(
            &duration,
            ThreadRef::downgrade(self.handler.current_thread()),
        )?;
        Ok(0)
    }
}

impl LinuxSyscallHandler {
    pub fn new() -> Self {
        Self {
            handler: SyscallHandler::new(),
        }
    }
}
