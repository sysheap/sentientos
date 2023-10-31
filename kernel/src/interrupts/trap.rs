use core::panic;

use crate::{
    interrupts::plic::{self, InterruptSource},
    io::uart,
    print, println,
    processes::{scheduler, timer},
};

use super::trap_cause::interrupt::*;
use super::trap_cause::{exception::*, InterruptCause};

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
        loop {}
        handle_exception(cause, stval, sepc, trap_frame);
    }
}

fn handle_exception(cause: InterruptCause, stval: usize, sepc: usize, trap_frame: &TrapFrame) {
    match cause.get_exception_code() {
        INSTRUCTION_ADDRESS_MISALIGNED => {
            panic!(
                "Instruction address misaligned! (stval: 0x{:x}) (sepc: 0x{:x})",
                stval, sepc
            );
        }
        INSTRUCTION_ACCESS_FAULT => {
            panic!(
                "Instruction access fault! (stval: 0x{:x}) (sepc: 0x{:x})",
                stval, sepc
            );
        }
        ILLEGAL_INSTRUCTION => {
            panic!(
                "Illegal instruction! (stval: 0x{:x}) (sepc: 0x{:x})",
                stval, sepc
            );
        }
        BREAKPOINT => {
            panic!("Breakpoint! (stval: 0x{:x}) (sepc: 0x{:x})", stval, sepc);
        }
        LOAD_ADDRESS_MISALIGNED => {
            panic!(
                "Load address misaligned! (stval: 0x{:x}) (sepc: 0x{:x})",
                stval, sepc
            );
        }
        LOAD_ACCESS_FAULT => {
            panic!(
                "Load access fault! (stval: 0x{:x}) (sepc: 0x{:x})",
                stval, sepc
            );
        }
        STORE_AMO_ADDRESS_MISALIGNED => {
            panic!(
                "Store/AMO address misaligned! (stval: 0x{:x}) (sepc: 0x{:x})",
                stval, sepc
            );
        }
        STORE_AMO_ACCESS_FAULT => {
            panic!(
                "Store/AMO access fault! (stval: 0x{:x}) (sepc: 0x{:x})",
                stval, sepc
            );
        }
        ENVIRONMENT_CALL_FROM_U_MODE => {
            panic!(
                "Environment call from U-mode! (stval: 0x{:x}) (sepc: 0x{:x})",
                stval, sepc
            );
        }
        ENVIRONMENT_CALL_FROM_S_MODE => {
            panic!(
                "Environment call from S-mode! (stval: 0x{:x}) (sepc: 0x{:x})",
                stval, sepc
            );
        }
        ENVIRONMENT_CALL_FROM_M_MODE => {
            panic!(
                "Environment call from M-mode! (stval: 0x{:x}) (sepc: 0x{:x})",
                stval, sepc
            );
        }
        _ => {
            panic!(
                "Unknown exception! (Name: {}) (stval: 0x{:x}) (sepc: 0x{:x})",
                cause.get_reason(),
                stval,
                sepc
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
