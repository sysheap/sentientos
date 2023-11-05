use core::ptr::slice_from_raw_parts;

use alloc::string::String;
use common::syscalls::{
    kernel::Syscalls, trap_frame::TrapFrame, userpointer::Userpointer, SYSCALL_INVALID_PROGRAM,
    SYSCALL_INVALID_PTR, SYSCALL_SUCCESS, SYSCALL_WAIT,
};

use crate::{
    debug, io::stdin_buf::STDIN_BUFFER,
    memory::page_tables::translate_userspace_address_to_physical_address, print,
    processes::scheduler,
};

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

    #[allow(non_snake_case)]
    fn EXECUTE(&self, name: Userpointer<u8>, length: usize) -> isize {
        // Check validity of userspointer before using it
        let physical_address = translate_userspace_address_to_physical_address(name.get());

        if let Some(physical_address) = physical_address {
            let slice = unsafe { &*slice_from_raw_parts(physical_address, length) };
            let mut name = String::with_capacity(length);
            for c in slice {
                name.push(*c as char);
            }

            if scheduler::schedule_program(&name) {
                SYSCALL_SUCCESS
            } else {
                SYSCALL_INVALID_PROGRAM
            }
        } else {
            SYSCALL_INVALID_PTR
        }
    }
}

pub fn handle_syscall(trap_frame: &mut TrapFrame) -> isize {
    let handler = SyscallHandler;
    SyscallHandler::handle(&handler, trap_frame)
}
