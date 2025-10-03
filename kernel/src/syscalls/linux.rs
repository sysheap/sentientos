use core::ffi::{c_int, c_uint, c_ulong};

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
    syscall_types::{
        _NSIG, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK, SIGKILL, SIGSTOP, pollfd, sigaction, sigset_t,
        stack_t, timespec,
    },
};

linux_syscalls! {
    SYSCALL_NR_EXIT_GROUP => exit_group(status: c_int);
    SYSCALL_NR_PPOLL => ppoll(fds: *mut pollfd, n: c_uint, to: Option<*const timespec>, mask: Option<*const sigset_t>);
    SYSCALL_NR_RT_SIGACTION => rt_sigaction(sig: c_uint, act: Option<*const sigaction>, oact: Option<*mut sigaction>, sigsetsize: usize);
    SYSCALL_NR_RT_SIGPROCMASK => rt_sigprocmask(how: c_uint, set: Option<*const sigset_t>, oldset: Option<*mut sigset_t>, sigsetsize: usize);
    SYSCALL_NR_SET_TID_ADDRESS => set_tid_address(tidptr: *mut c_int);
    SYSCALL_NR_SIGALTSTACK => sigaltstack(uss: Option<*const stack_t>, uoss: Option<*mut stack_t>);
    SYSCALL_NR_WRITE => write(fd: c_int, buf: *const u8, count: usize);
    SYSCALL_NR_NANOSLEEP => nanosleep(duration: *const timespec, rem: Option<*const timespec>);
    SYSCALL_NR_BRK => brk(brk: c_ulong);
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
        let mask = mask.validate_ptr()?;

        let old_mask = if let Some(mask) = mask {
            Some(
                self.handler
                    .current_thread()
                    .with_lock(|mut t| t.set_sigset(mask)),
            )
        } else {
            None
        };

        let to = to.validate_ptr()?;
        if let Some(to) = to {
            assert_eq!(to.tv_sec, 0, "ppoll with timeout not yet implemented");
            assert_eq!(to.tv_nsec, 0, "ppoll with timeout not yet implemented");
        }

        let fds = fds.validate_slice(n as usize)?;

        for fd in fds {
            assert!(
                matches!(fd.fd, 0..=2),
                "Only stdin, stdout, and stderr is supported currently"
            );
            assert_eq!(
                fd.events, 0,
                "No further events are supported at the moment"
            );
        }

        if let Some(mask) = old_mask {
            self.handler
                .current_thread()
                .with_lock(|mut t| t.set_sigset(mask));
        }

        Ok(0)
    }

    fn rt_sigaction(
        &mut self,
        sig: c_uint,
        act: LinuxUserspaceArg<Option<*const sigaction>>,
        oact: LinuxUserspaceArg<Option<*mut sigaction>>,
        sigsetsize: usize,
    ) -> Result<isize, Errno> {
        if sigsetsize != core::mem::size_of::<sigset_t>()
            || matches!(sig, SIGKILL | SIGSTOP)
            || sig >= _NSIG
        {
            return Err(Errno::EINVAL);
        }

        let old_act = if let Some(act) = act.validate_ptr()? {
            self.handler
                .current_thread()
                .with_lock(|mut t| t.set_sigaction(sig, act))
        } else {
            self.handler
                .current_thread()
                .with_lock(|t| t.get_sigaction(sig))
        }?;

        oact.write_if_not_none(old_act)?;

        Ok(0)
    }

    fn rt_sigprocmask(
        &mut self,
        how: c_uint,
        set: LinuxUserspaceArg<Option<*const sigset_t>>,
        oldset: LinuxUserspaceArg<Option<*mut sigset_t>>,
        sigsetsize: usize,
    ) -> Result<isize, Errno> {
        if sigsetsize != core::mem::size_of::<sigset_t>() {
            return Err(Errno::EINVAL);
        }

        let new_set = set.validate_ptr()?;

        let old_set_in_thread = if let Some(new_set) = new_set {
            self.handler.current_thread().with_lock(|mut t| {
                let mut current_set = t.get_sigset();
                match how {
                    SIG_BLOCK => current_set.sig[0] |= new_set.sig[0],
                    SIG_UNBLOCK => current_set.sig[0] &= !new_set.sig[0],
                    SIG_SETMASK => current_set.sig[0] = new_set.sig[0],
                    _ => {
                        return Err(Errno::EINVAL);
                    }
                }
                Ok(t.set_sigset(current_set))
            })?
        } else {
            self.handler.current_thread().with_lock(|t| t.get_sigset())
        };

        oldset.write_if_not_none(old_set_in_thread)?;

        Ok(0)
    }

    fn sigaltstack(
        &mut self,
        uss: LinuxUserspaceArg<Option<*const stack_t>>,
        uoss: LinuxUserspaceArg<Option<*mut stack_t>>,
    ) -> Result<isize, Errno> {
        let uss = uss.validate_ptr()?;
        self.handler.current_thread().with_lock(|mut t| {
            let old = t.get_sigaltstack();
            if let Some(uss) = uss {
                t.set_sigaltstack(&uss);
            }
            uoss.write_if_not_none(old)?;
            Ok::<(), Errno>(())
        })?;
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

    fn brk(&mut self, brk: c_ulong) -> Result<isize, headers::errno::Errno> {
        self.handler
            .current_process()
            .with_lock(|mut p| Ok(p.brk(brk as usize) as isize))
    }
}

impl LinuxSyscallHandler {
    pub fn new() -> Self {
        Self {
            handler: SyscallHandler::new(),
        }
    }
}
