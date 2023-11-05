use common::syscalls::{kernel::Syscalls, trap_frame::TrapFrame, SYSCALL_SUCCESS, SYSCALL_WAIT};

use crate::{debug, io::stdin_buf::STDIN_BUFFER, print, processes::scheduler};

struct SyscallHandler;

impl common::syscalls::kernel::Syscalls for SyscallHandler {
    #[allow(non_snake_case)]
    fn WRITE_CHAR(&self, c: u8) -> isize {
        print!("{}", c as char);
        SYSCALL_SUCCESS
    }

    #[allow(non_snake_case)]
    fn READ_CHAR(&self) -> isize {
        let mut stdin = STDIN_BUFFER.lock();
        if let Some(c) = stdin.pop() {
            c as isize
        } else {
            SYSCALL_WAIT
        }
    }

    #[allow(non_snake_case)]
    fn EXIT(&self, status: isize) -> isize {
        debug!("Exit process with status: {}\n", status);
        scheduler::kill_current_process();
        SYSCALL_SUCCESS
    }
}

pub fn handle_syscall(trap_frame: &mut TrapFrame) -> isize {
    let handler = SyscallHandler;
    SyscallHandler::handle(&handler, trap_frame)
}
