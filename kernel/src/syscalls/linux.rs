use core::ffi::c_int;

use crate::{cpu::Cpu, print};
use common::{
    constructable::Constructable,
    pointer::FatPointer,
    syscalls::{
        kernel::KernelSyscalls,
        trap_frame::{Register, TrapFrame},
    },
};

use crate::syscalls::{handler::SyscallHandler, validator::UserspaceArgument};
use headers::syscalls::*;

// TODO: This should be better organized (preferably with a syscall macro like the sentientos syscall)
// Also argument verification is not implemented right now. Let's get first going
// and then add security.

pub struct LinuxSyscallHandler<'a> {
    handler: SyscallHandler,
    trap_frame: &'a TrapFrame,
}

impl<'a> LinuxSyscallHandler<'a> {
    pub fn new(trap_frame: &'a TrapFrame) -> Self {
        Self {
            handler: SyscallHandler::new(),
            trap_frame,
        }
    }

    pub fn handle(&mut self) -> isize {
        let nr = self.trap_frame[Register::a7];
        let arg1 = self.trap_frame[Register::a0];
        let arg2 = self.trap_frame[Register::a1];
        let arg3 = self.trap_frame[Register::a2];
        match nr {
            SYSCALL_NR_WRITE => self.handle_write(arg1 as i32, arg2 as *const u8, arg3),
            SYSCALL_NR_EXIT_GROUP => self.handle_exit_group(arg1 as c_int),
            SYSCALL_NR_SET_TID_ADDRESS => self.handle_set_tid_address(arg1 as *const c_int),
            SYSCALL_NR_PPOLL => self.handle_ppoll_time32(),
            SYSCALL_NR_RT_SIGACTION => self.handle_rt_sigaction(),
            SYSCALL_NR_SIGALTSTACK => self.handle_sigaltstack(),
            SYSCALL_NR_RT_SIGPROCMASK => self.handle_rt_sigprocmask(),
            SYSCALL_NR_TKILL => self.handle_tkill(),
            SYSCALL_NR_BRK => self.handle_brk(),
            _ => {
                panic!("Linux Syscall Nr {nr} at {:#x}", Cpu::read_sepc());
            }
        }
    }

    fn handle_tkill(&self) -> isize {
        0
    }

    fn handle_brk(&self) -> isize {
        0
    }

    fn handle_rt_sigprocmask(&self) -> isize {
        0
    }

    fn handle_sigaltstack(&self) -> isize {
        0
    }

    fn handle_rt_sigaction(&self) -> isize {
        0
    }

    fn handle_ppoll_time32(&self) -> isize {
        0
    }

    fn handle_set_tid_address(&self, _tidptr: *const c_int) -> isize {
        0
    }

    fn handle_exit_group(&mut self, status: c_int) -> isize {
        self.handler
            .sys_exit(UserspaceArgument::new(status as isize));
        0
    }

    fn handle_write(&mut self, fd: c_int, buf: *const u8, len: usize) -> isize {
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
}
