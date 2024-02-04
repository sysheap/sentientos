mod validator;

use core::ptr::slice_from_raw_parts;

use alloc::string::String;
use common::syscalls::{
    kernel::KernelSyscalls, userspace_argument::UserspaceArgument, SYSCALL_INVALID_PID,
    SYSCALL_INVALID_PROGRAM, SYSCALL_INVALID_PTR, SYSCALL_SUCCESS, SYSCALL_WAIT,
};

use crate::{
    debug,
    io::stdin_buf::STDIN_BUFFER,
    print,
    processes::scheduler::{self, get_current_process_expect, let_current_process_wait_for},
    syscalls::validator::UserspaceArgumentValidator,
};

use self::validator::FailibleUserspaceArgumentValidator;

struct SyscallHandler;

impl KernelSyscalls for SyscallHandler {
    fn sys_write_char(c: UserspaceArgument<char>) -> isize {
        print!("{}", c.validate());
        SYSCALL_SUCCESS
    }

    fn sys_read_char() -> isize {
        let mut stdin = STDIN_BUFFER.lock();
        if let Some(c) = stdin.pop() {
            c as isize
        } else {
            SYSCALL_WAIT
        }
    }

    fn sys_exit(status: UserspaceArgument<isize>) -> isize {
        debug!("Exit process with status: {}\n", status.validate());
        scheduler::kill_current_process();
        SYSCALL_SUCCESS
    }

    fn sys_execute(name: UserspaceArgument<&u8>, length: UserspaceArgument<usize>) -> isize {
        // TODO: Move it into validator
        // Check validity of userspointer before using it
        let current_process = get_current_process_expect();
        let current_process = current_process.borrow();
        let physical_address = current_process
            .get_page_table()
            .translate_userspace_address_to_physical_address(name.validate().unwrap());

        let length = length.validate();

        if let Some(physical_address) = physical_address {
            let slice = unsafe { &*slice_from_raw_parts(physical_address, length) };
            let mut name = String::with_capacity(length);
            for c in slice {
                name.push(*c as char);
            }

            if let Some(pid) = scheduler::schedule_program(&name) {
                pid as isize
            } else {
                SYSCALL_INVALID_PROGRAM
            }
        } else {
            SYSCALL_INVALID_PTR
        }
    }

    fn sys_wait(pid: UserspaceArgument<u64>) -> isize {
        if let_current_process_wait_for(pid.validate()) {
            SYSCALL_SUCCESS
        } else {
            SYSCALL_INVALID_PID
        }
    }

    fn sys_mmap_pages(number_of_pages: UserspaceArgument<usize>) -> isize {
        let current_process = get_current_process_expect();
        let mut current_process = current_process.borrow_mut();
        current_process.mmap_pages(number_of_pages.validate()) as isize
    }
}

pub fn handle_syscall(nr: usize, arg1: usize, arg2: usize) -> usize {
    SyscallHandler::dispatch(nr, arg1, arg2)
}
