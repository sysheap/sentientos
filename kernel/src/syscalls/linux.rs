use core::ffi::c_int;

use crate::{cpu::Cpu, info, print};
use common::{
    constructable::Constructable,
    pointer::FatPointer,
    syscalls::{
        kernel::KernelSyscalls,
        trap_frame::{Register, TrapFrame},
    },
};

use crate::syscalls::{handler::SyscallHandler, validator::UserspaceArgument};

pub fn handle(trap_frame: &TrapFrame) -> isize {
    let nr = trap_frame[Register::a7];
    let handler = SyscallHandler::new();
    match nr {
        64 => handle_write(trap_frame, handler),
        94 => handle_exit_group(trap_frame, handler),
        96 => handle_set_tid_address(trap_frame, handler),
        73 => handle_ppoll_time32(),
        _ => {
            info!("Linux Syscall Nr {nr} at {:#x}", Cpu::read_sepc());
            0
        }
    }
}

fn handle_ppoll_time32() -> isize {
    info!("PPOLL_TIME32");
    0
}

fn handle_set_tid_address(trap_frame: &TrapFrame, mut _handler: SyscallHandler) -> isize {
    info!(
        "Set TID to {:p} (NOT IMPLEMENTED)",
        trap_frame[Register::a0] as *const c_int
    );
    0
}

fn handle_exit_group(trap_frame: &TrapFrame, mut handler: SyscallHandler) -> isize {
    let status = trap_frame[Register::a0];
    handler.sys_exit(UserspaceArgument::new(status as isize));
    0
}

fn handle_write(trap_frame: &TrapFrame, mut handler: SyscallHandler) -> isize {
    let fd = trap_frame[Register::a0];
    let buf = trap_frame[Register::a1];
    let len = trap_frame[Register::a2];

    if fd != 1 && fd != 2 {
        return -1;
    }

    if fd == 2 {
        print!("ERROR: ");
    }

    let result = handler.sys_write(UserspaceArgument::new(FatPointer::new(
        buf as *const u8,
        len,
    )));

    if result.is_ok() { len as isize } else { -1 }
}
