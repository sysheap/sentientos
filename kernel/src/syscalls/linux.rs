use crate::{
    io::stdin_buf::ReadStdin,
    memory::{PAGE_SIZE, page_tables::XWRMode},
    print,
    processes::{process::ProcessRef, timer},
    syscalls::{handler::SyscallHandler, macros::linux_syscalls, validator::UserspaceArgument},
};
use alloc::{string::String, vec::Vec};
use common::{
    constructable::Constructable,
    syscalls::{
        kernel::KernelSyscalls,
        trap_frame::{Register, TrapFrame},
    },
};
use core::ffi::{c_int, c_uint, c_ulong};
use headers::{
    errno::Errno,
    syscall_types::{
        _NSIG, MAP_ANONYMOUS, MAP_FIXED, MAP_PRIVATE, PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE,
        SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK, SIGKILL, SIGSTOP, TIOCGWINSZ, iovec, pollfd,
        sigaction, sigset_t, stack_t, timespec,
    },
};

linux_syscalls! {
    SYSCALL_NR_BRK => brk(brk: c_ulong);
    SYSCALL_NR_CLOSE => close(fd: c_int);
    SYSCALL_NR_EXIT_GROUP => exit_group(status: c_int);
    SYSCALL_NR_GETTID => gettid();
    SYSCALL_NR_IOCTL => ioctl(fd: c_int, op: c_uint);
    SYSCALL_NR_MMAP => mmap(addr: usize, length: usize, prot: c_uint, flags: c_uint, fd: c_int, offset: isize);
    SYSCALL_NR_MUNMAP => munmap(addr: usize, length: usize);
    SYSCALL_NR_NANOSLEEP => nanosleep(duration: *const timespec, rem: Option<*const timespec>);
    SYSCALL_NR_PPOLL => ppoll(fds: *mut pollfd, n: c_uint, to: Option<*const timespec>, mask: Option<*const sigset_t>);
    SYSCALL_NR_PRCTL => prctl();
    SYSCALL_NR_READ => read(fd: c_int, buf: *mut u8, count: usize);
    SYSCALL_NR_RT_SIGACTION => rt_sigaction(sig: c_uint, act: Option<*const sigaction>, oact: Option<*mut sigaction>, sigsetsize: usize);
    SYSCALL_NR_RT_SIGPROCMASK => rt_sigprocmask(how: c_uint, set: Option<*const sigset_t>, oldset: Option<*mut sigset_t>, sigsetsize: usize);
    SYSCALL_NR_SET_TID_ADDRESS => set_tid_address(tidptr: *mut c_int);
    SYSCALL_NR_SIGALTSTACK => sigaltstack(uss: Option<*const stack_t>, uoss: Option<*mut stack_t>);
    SYSCALL_NR_WRITEV => writev(fd: c_int, iov: *const iovec, iovcnt: c_int);
    SYSCALL_NR_WRITE => write(fd: c_int, buf: *const u8, count: usize);
}

pub struct LinuxSyscallHandler {
    handler: SyscallHandler,
}

impl LinuxSyscalls for LinuxSyscallHandler {
    async fn read(
        &mut self,
        fd: c_int,
        buf: LinuxUserspaceArg<*mut u8>,
        count: usize,
    ) -> Result<isize, headers::errno::Errno> {
        if fd != 0 {
            return Err(Errno::EBADF);
        }

        let data = ReadStdin::new(count).await;

        assert!(data.len() <= count, "Read more than requested");

        buf.write_slice(&data)?;

        Ok(data.len() as isize)
    }

    async fn write(
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

    async fn exit_group(&mut self, status: c_int) -> Result<isize, Errno> {
        self.handler
            .sys_exit(UserspaceArgument::new(status as isize));
        Ok(0)
    }

    async fn set_tid_address(
        &mut self,
        tidptr: LinuxUserspaceArg<*mut c_int>,
    ) -> Result<isize, Errno> {
        self.handler.current_thread().with_lock(|mut t| {
            t.set_clear_child_tid((&tidptr).into());
        });
        Ok(0)
    }

    async fn ppoll(
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

    async fn rt_sigaction(
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

    async fn rt_sigprocmask(
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

    async fn sigaltstack(
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

    async fn nanosleep(
        &mut self,
        duration: LinuxUserspaceArg<*const timespec>,
        _rem: LinuxUserspaceArg<Option<*const timespec>>,
    ) -> Result<isize, Errno> {
        let duration = duration.validate_ptr()?;
        if duration.tv_sec < 0 || !(0..999999999).contains(&duration.tv_nsec) {
            return Err(Errno::EINVAL);
        }
        timer::sleep(&duration)?.await;
        Ok(0)
    }

    async fn brk(&mut self, brk: c_ulong) -> Result<isize, headers::errno::Errno> {
        self.handler
            .current_process()
            .with_lock(|mut p| Ok(p.brk(brk as usize) as isize))
    }

    async fn mmap(
        &mut self,
        addr: usize,
        length: usize,
        prot: c_uint,
        flags: c_uint,
        fd: c_int,
        offset: isize,
    ) -> Result<isize, headers::errno::Errno> {
        assert_eq!(
            flags & !(MAP_ANONYMOUS | MAP_PRIVATE | MAP_FIXED),
            0,
            "Only this flags are implemented so far."
        );
        assert_eq!(
            flags & (MAP_ANONYMOUS | MAP_PRIVATE),
            MAP_ANONYMOUS | MAP_PRIVATE,
            "File backed mappings and shared mappings are not supported yet."
        );
        assert_eq!(fd, -1, "fd must be -1 when working in MAP_ANONYMOUS");
        assert_eq!(
            offset, 0,
            "offset must be null when working with MAP_ANONYMOUS"
        );
        assert_eq!(
            length % PAGE_SIZE,
            0,
            "Length must be dividable through PAGE_SIZE"
        );
        if (flags & MAP_FIXED) > 0 && addr == 0 {
            return Err(Errno::EINVAL);
        }
        if length == 0 {
            return Err(Errno::EINVAL);
        }
        // Handle special PROT_NONE case and map it to the null pointer
        if prot == PROT_NONE {
            return self.handler.current_process().with_lock(|mut p| {
                if p.get_page_table().is_mapped(addr..addr + length) {
                    return Err(Errno::EEXIST);
                }
                p.get_page_table_mut().map_userspace(
                    addr,
                    0,
                    length / PAGE_SIZE,
                    XWRMode::ReadOnly,
                    "PROT_NONE".into(),
                );
                Ok(addr as isize)
            });
        }
        let permission = match prot {
            PROT_EXEC => XWRMode::ExecuteOnly,
            PROT_READ => XWRMode::ReadOnly,
            x if x == (PROT_READ | PROT_WRITE) => XWRMode::ReadWrite,
            _ => return Err(Errno::EINVAL),
        };
        self.handler.current_process().with_lock(|mut p| {
            if (flags & MAP_FIXED) > 0 {
                if p.get_page_table().is_mapped(addr..addr + length) {
                    return Err(Errno::EEXIST);
                }
                let ptr = p.mmap_pages_with_address(length / PAGE_SIZE, addr, permission);
                return Ok(ptr as isize);
            }
            if addr == 0 || p.get_page_table().is_mapped(addr..addr + length) {
                return Ok(p.mmap_pages(length / PAGE_SIZE, permission) as isize);
            }
            Ok(p.mmap_pages_with_address(length / PAGE_SIZE, addr, permission) as isize)
        })
    }

    async fn munmap(
        &mut self,
        _addr: usize,
        _length: usize,
    ) -> Result<isize, headers::errno::Errno> {
        // Ignore munmap for now
        Ok(0)
    }

    async fn prctl(&mut self) -> Result<isize, headers::errno::Errno> {
        // We dont support any of prctl right now
        Err(Errno::EINVAL)
    }

    async fn ioctl(&mut self, fd: c_int, op: c_uint) -> Result<isize, headers::errno::Errno> {
        if fd > 2 {
            return Err(Errno::EBADFD);
        }
        if op == TIOCGWINSZ && (fd == 1 || fd == 2) {
            return Err(Errno::ENOTTY);
        }
        Err(Errno::EINVAL)
    }

    async fn writev(
        &mut self,
        fd: c_int,
        iov: LinuxUserspaceArg<*const iovec>,
        iovcnt: c_int,
    ) -> Result<isize, headers::errno::Errno> {
        if fd != 1 && fd != 2 {
            return Err(Errno::EBADF);
        }

        let iov = iov.validate_slice(iovcnt as usize)?;
        let mut data = Vec::new();

        for io in iov {
            let buf = LinuxUserspaceArg::<*const u8>::new(io.iov_base as usize, self.get_process());
            let mut buf = buf.validate_slice(io.iov_len as usize)?;
            data.append(&mut buf);
        }

        let len = data.len();
        print!("{}", String::from_utf8_lossy_owned(data));

        Ok(len as isize)
    }

    async fn close(
        &mut self,
        _fd: <c_int as crate::syscalls::macros::NeedsUserSpaceWrapper>::Wrapped,
    ) -> Result<isize, headers::errno::Errno> {
        // TODO: Implement when we really manage fd objects
        Ok(0)
    }

    async fn gettid(&mut self) -> Result<isize, headers::errno::Errno> {
        Ok(self.handler.current_tid().0 as isize)
    }
}

impl LinuxSyscallHandler {
    pub fn new() -> Self {
        Self {
            handler: SyscallHandler::new(),
        }
    }
}
