use core::panic;

use common::syscalls::trap_frame::{Register, TrapFrame};

use crate::{
    cpu, debug,
    interrupts::plic::{self, InterruptSource},
    io::uart,
    memory::page_tables,
    print, println,
    processes::{scheduler, timer},
    syscalls::handle_syscall,
};

use super::trap_cause::InterruptCause;
use super::trap_cause::{exception::ENVIRONMENT_CALL_FROM_U_MODE, interrupt::*};

#[no_mangle]
extern "C" fn supervisor_mode_trap(
    cause: InterruptCause,
    stval: usize,
    sepc: usize,
    trap_frame: &mut TrapFrame,
) {
    debug!(
        "Supervisor mode trap occurred! (sepc: {:x?}) (cause: {:?})\nTrap Frame: {:?}",
        sepc,
        cause.get_reason(),
        trap_frame
    );
    if cause.is_interrupt() {
        handle_interrupt(cause, stval, sepc, trap_frame);
    } else {
        handle_exception(cause, stval, sepc, trap_frame);
    }
}

fn handle_exception(cause: InterruptCause, stval: usize, sepc: usize, trap_frame: &mut TrapFrame) {
    match cause.get_exception_code() {
        ENVIRONMENT_CALL_FROM_U_MODE => {
            trap_frame[Register::a0] = handle_syscall(trap_frame) as usize;
            cpu::write_sepc(sepc + 4); // Skip the ecall instruction
        }
        _ => {
            panic!(
                "Unhandled exception! (Name: {}) (Exception code: {}) (stval: 0x{:x}) (sepc: 0x{:x}) (From Userspace: {})\nTrap Frame: {:?}",
                cause.get_reason(),
                cause.get_exception_code(),
                stval,
                sepc,
                page_tables::is_userspace_address(sepc),
                trap_frame
            );
        }
    }
}

fn handle_interrupt(cause: InterruptCause, stval: usize, sepc: usize, trap_frame: &TrapFrame) {
    match cause.get_exception_code() {
        SUPERVISOR_TIMER_INTERRUPT => handle_supervisor_timer_interrupt(),
        SUPERVISOR_EXTERNAL_INTERRUPT => handle_external_interrupt(),
        _ => {
            panic!("Unknwon interrupt! (Name: {})", cause.get_reason());
        }
    }
}

fn handle_supervisor_timer_interrupt() {
    debug!("Supervisor timer interrupt occurred!");
    timer::set_timer(1000);
    scheduler::schedule();
}

fn handle_external_interrupt() {
    debug!("External interrupt occurred!");
    let plic_interrupt = plic::get_next_pending().expect("There should be a pending interrupt.");
    assert!(
        plic_interrupt == InterruptSource::Uart,
        "Plic interrupt should be uart."
    );

    let input = uart::read().expect("There should be input from the uart.");

    match input {
        8 => {
            // This is a backspace, so we
            // essentially have to write a space and
            // backup again:
            print!("{} {}", 8 as char, 8 as char);
        }
        10 | 13 => {
            // Newline or carriage-return
            println!();
        }
        _ => {
            print!("{}", input as char);
        }
    };

    plic::complete_interrupt(plic_interrupt);
}
