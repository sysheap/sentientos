use core::panic;

use crate::{
    interrupts::plic::{self, InterruptSource},
    io::uart,
    memory::page_tables,
    print, println,
    processes::{scheduler, timer},
};

use super::trap_cause::interrupt::*;
use super::trap_cause::InterruptCause;

#[repr(packed)]
pub struct TrapFrame {
    registers: [usize; 32],
    floating_registers: [usize; 32],
}

impl TrapFrame {
    const STACK_POINTER_REGISTER_INDEX: usize = 2;

    pub const fn zero() -> Self {
        Self {
            registers: [0; 32],
            floating_registers: [0; 32],
        }
    }

    pub fn set_stack_pointer(&mut self, stack_pointer: usize) {
        self.registers[TrapFrame::STACK_POINTER_REGISTER_INDEX] = stack_pointer;
    }
}

#[no_mangle]
extern "C" fn supervisor_mode_trap(
    cause: InterruptCause,
    stval: usize,
    sepc: usize,
    trap_frame: &TrapFrame,
) {
    if cause.is_interrupt() {
        handle_interrupt(cause, stval, sepc, trap_frame);
    } else {
        handle_exception(cause, stval, sepc, trap_frame);
    }
}

fn handle_exception(cause: InterruptCause, stval: usize, sepc: usize, trap_frame: &TrapFrame) {
    match cause.get_exception_code() {
        _ => {
            panic!(
                "Unhandled exception! (Name: {}) (Exception code: {}) (stval: 0x{:x}) (sepc: 0x{:x}) (From Userspace: {})",
                cause.get_reason(),
                cause.get_exception_code(),
                stval,
                sepc,
                page_tables::is_userspace_address(sepc)
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
    println!("Supervisor timer interrupt occurred!");
    timer::set_timer(1000);
    scheduler::schedule();
}

fn handle_external_interrupt() {
    print!("External interrupt occurred!");
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
