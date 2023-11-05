use common::syscalls::{kernel::Syscalls, trap_frame::TrapFrame, SYSCALL_SUCCESS};

use crate::println;

struct SyscallHandler;

impl common::syscalls::kernel::Syscalls for SyscallHandler {
    #[allow(non_snake_case)]
    fn WRITE_CHAR(c: u8) -> isize {
        println!("Process prints: {}", c as char);
        SYSCALL_SUCCESS
    }
}

pub fn handle_syscall(trap_frame: &mut TrapFrame) -> isize {
    SyscallHandler::handle(trap_frame)
}
