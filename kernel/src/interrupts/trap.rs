use super::trap_cause::{exception::ENVIRONMENT_CALL_FROM_U_MODE, InterruptCause};
use crate::{
    cpu::{self},
    debug,
    interrupts::plic::{self, InterruptSource},
    io::{stdin_buf::STDIN_BUFFER, uart},
    memory::linker_information::LinkerInformation,
    processes::scheduler::{self, get_current_process, schedule},
    syscalls::handle_syscall,
    warn,
};
use common::syscalls::trap_frame::{Register, TrapFrame};
use core::panic;

#[no_mangle]
extern "C" fn handle_timer_interrupt() {
    scheduler::schedule();
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
        3 => crate::processes::scheduler::send_ctrl_c(),
        4 => crate::debugging::dump_current_state(),
        _ => STDIN_BUFFER.lock().push(input),
    }
}

#[no_mangle]
extern "C" fn handle_exception(
    cause: InterruptCause,
    stval: usize,
    sepc: usize,
    trap_frame: &mut TrapFrame,
) {
    match cause.get_exception_code() {
        ENVIRONMENT_CALL_FROM_U_MODE => {
            let nr = trap_frame[Register::a0];
            let arg1 = trap_frame[Register::a1];
            let arg2 = trap_frame[Register::a2];
            let arg3 = trap_frame[Register::a3];
            let ret = handle_syscall(nr, arg1, arg2, arg3);
            if let Some((ret1, ret2)) = ret {
                trap_frame[Register::a0] = ret1;
                trap_frame[Register::a1] = ret2;
                cpu::write_sepc(sepc + 4); // Skip the ecall instruction
            }
            // In case our current process was set to waiting state we need to reschedule
            if let Some(process) = get_current_process()
                && process.lock().get_state().is_waiting()
            {
                schedule();
            }
        }
        _ => {
            if cause.is_stack_overflow(stval) {
                let guard_range = LinkerInformation::__start_stack_overflow_guard()
                    ..LinkerInformation::__start_kernel_stack();
                warn!(
                    "DANGER! STACK OVERFLOW DETECTED! stval={:p} inside guard page {:p}-{:p}",
                    stval as *const (),
                    guard_range.start as *const (),
                    guard_range.end as *const ()
                );
            }
            let current_process = get_current_process();
            if let Some(current_process) = current_process {
                let current_process = current_process.lock();
                panic!(
                    "Unhandled exception!\nName: {}\nException code: {}\nstval: 0x{:x}\nsepc: 0x{:x}\nFrom Userspace: {}\nProcess name: {}\nTrap Frame: {:?}",
                    cause.get_reason(),
                    cause.get_exception_code(),
                    stval,
                    sepc,
                    current_process.get_page_table().is_userspace_address(sepc),
                    current_process.get_name(),
                    trap_frame
                );
            } else {
                panic!(
                    "Unhandled exception!\nName: {}\nException code: {}\nstval: 0x{:x}\nsepc: 0x{:x}\nFrom Userspace: {}\nProcess name: {}\nTrap Frame: {:?}",
                    cause.get_reason(),
                    cause.get_exception_code(),
                    stval,
                    sepc,
                    false,
                    "No scheduled process",
                    trap_frame
                );
            }
        }
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
