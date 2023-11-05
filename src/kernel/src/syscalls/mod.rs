use common::syscalls::{kernel::Syscalls, trap_frame::TrapFrame, SYSCALL_SUCCESS};

use crate::{debug, print, processes::scheduler};

struct SyscallHandler;

impl common::syscalls::kernel::Syscalls for SyscallHandler {
    #[allow(non_snake_case)]
    fn WRITE_CHAR(c: u8) -> isize {
        print!("{}", c as char);
        SYSCALL_SUCCESS
    }

    #[allow(non_snake_case)]
    fn EXIT(status: isize) -> isize {
        debug!("Exit process with status: {}\n", status);
        scheduler::kill_current_process();
        SYSCALL_SUCCESS
    }
}

pub fn handle_syscall(trap_frame: &mut TrapFrame) -> isize {
    SyscallHandler::handle(trap_frame)
}
