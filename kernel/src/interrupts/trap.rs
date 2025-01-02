use super::trap_cause::{exception::ENVIRONMENT_CALL_FROM_U_MODE, InterruptCause};
use crate::{
    cpu::{self},
    debug,
    interrupts::plic::{self, InterruptSource},
    io::{stdin_buf::STDIN_BUFFER, uart},
    memory::linker_information::LinkerInformation,
    processes::{
        process::ProcessState,
        scheduler::{self},
    },
    syscalls::{self},
    warn,
};
use common::syscalls::trap_frame::{Register, TrapFrame};
use core::panic;

#[no_mangle]
extern "C" fn handle_timer_interrupt() {
    scheduler::THE.lock().schedule();
}

#[no_mangle]
fn handle_external_interrupt() {
    debug!("External interrupt occurred!");
    let plic_interrupt = plic::get_next_pending().expect("There should be a pending interrupt.");
    assert!(
        plic_interrupt == InterruptSource::Uart,
        "Plic interrupt should be uart."
    );

    let input = uart::read().expect("There should be input from the uart.");

    plic::complete_interrupt(plic_interrupt);

    match input {
        3 => scheduler::THE.lock().send_ctrl_c(),
        4 => crate::debugging::dump_current_state(),
        _ => STDIN_BUFFER.lock().push(input),
    }
}

fn handle_syscall(sepc: usize, trap_frame: &mut TrapFrame) {
    let nr = trap_frame[Register::a0];
    let arg = trap_frame[Register::a1];
    let ret = trap_frame[Register::a2];

    let ret = syscalls::handle_syscall(nr, arg, ret);

    if let Some(ret) = ret {
        trap_frame[Register::a0] = ret as usize;
        cpu::write_sepc(sepc + 4); // Skip the ecall instruction
    }

    // In case our current process was set to waiting state we need to reschedule
    scheduler::THE.with_lock(|mut s| {
        if s.get_current_process().lock().get_state() == ProcessState::Waiting {
            s.schedule();
        }
    });
}

fn warn_on_stackoverflow(cause: InterruptCause, stval: usize) {
    if cause.is_stack_overflow(stval) {
        let guard_range = LinkerInformation::__start_stack_overflow_guard()
            ..LinkerInformation::__start_kernel_stack();
        warn!(
            "DANGER! STACK OVERFLOW DETECTED! stval={:p} inside guard page {:p}-{:p}",
            stval as *const (), guard_range.start as *const (), guard_range.end as *const ()
        );
    }
}

fn handle_unhandled_exception(
    cause: InterruptCause,
    stval: usize,
    sepc: usize,
    trap_frame: &mut TrapFrame,
) {
    let message= scheduler::THE.lock().get_current_process().with_lock(|p| {
        format!(
            "Unhandled exception!\nName: {}\nException code: {}\nstval: 0x{:x}\nsepc: 0x{:x}\nFrom Userspace: {}\nProcess name: {}\nTrap Frame: {:?}",
            cause.get_reason(),
            cause.get_exception_code(),
            stval,
            sepc,
            p.get_page_table().is_userspace_address(sepc),
            p.get_name(),
            trap_frame
        )
    });
    panic!("{}", message);
}

#[no_mangle]
extern "C" fn handle_exception(
    cause: InterruptCause,
    stval: usize,
    sepc: usize,
    trap_frame: &mut TrapFrame,
) {
    warn_on_stackoverflow(cause, stval);
    match cause.get_exception_code() {
        ENVIRONMENT_CALL_FROM_U_MODE => handle_syscall(sepc, trap_frame),
        _ => handle_unhandled_exception(cause, stval, sepc, trap_frame),
    }
}

#[no_mangle]
extern "C" fn handle_unimplemented(
    cause: InterruptCause,
    _stval: usize,
    sepc: usize,
    _trap_frame: &mut TrapFrame,
) {
    panic!(
        "Unimplemeneted trap occurred! (sepc: {:x?}) (cause: {:?})",
        sepc,
        cause.get_reason(),
    );
}
