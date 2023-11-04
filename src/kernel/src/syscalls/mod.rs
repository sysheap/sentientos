use common::syscalls::trap_frame::{Register, TrapFrame};

use crate::println;

const SYSCALL_NR_REG: Register = Register::a7;
const SYSCALL_ARG1_REG: Register = Register::a0;

pub fn handle_syscall(trap_frame: &mut TrapFrame) {
    let syscall_nr = trap_frame[SYSCALL_NR_REG];
    let arg1 = trap_frame[SYSCALL_ARG1_REG];

    match syscall_nr {
        0 => {
            println!("Process prints: {}", arg1 as u8 as char);
        }
        _ => {
            panic!("Unknown syscall nr: {}", syscall_nr);
        }
    }
}
