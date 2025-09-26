use super::trap_cause::{InterruptCause, exception::ENVIRONMENT_CALL_FROM_U_MODE};
use crate::{
    cpu::Cpu,
    debug, info,
    interrupts::plic::{self, InterruptSource},
    io::{stdin_buf::STDIN_BUFFER, uart},
    processes::thread::ThreadState,
    syscalls::{
        self,
        linux::{LinuxSyscallHandler, LinuxSyscalls},
    },
};
use common::syscalls::trap_frame::Register;
use core::panic;

#[unsafe(no_mangle)]
extern "C" fn get_process_satp_value() -> usize {
    Cpu::with_current_process(|p| p.get_page_table().get_satp_value_from_page_tables())
}

#[unsafe(no_mangle)]
extern "C" fn handle_timer_interrupt() {
    Cpu::with_scheduler(|s| s.schedule());
}

#[unsafe(no_mangle)]
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
        3 => Cpu::current().scheduler_mut().send_ctrl_c(),
        4 => crate::debugging::dump_current_state(),
        _ => STDIN_BUFFER.lock().push(input),
    }
}

fn handle_syscall() {
    let cpu = Cpu::current();
    let scheduler = cpu.scheduler();

    let trap_frame = *scheduler.trap_frame();
    let nr = trap_frame[Register::a7];
    let arg = trap_frame[Register::a1];
    let ret = trap_frame[Register::a2];

    drop(cpu);

    let ret = if (1 << 63) & nr > 0 {
        // We might need to get the current cpu again in handle_syscall
        syscalls::handle_syscall(nr, arg, ret)
    } else {
        let mut handler = LinuxSyscallHandler::new();
        let result = handler.handle(&trap_frame);
        let mut cpu = Cpu::current();
        let scheduler = cpu.scheduler_mut();
        let trap_frame = scheduler.trap_frame_mut();
        let ret = match result {
            Ok(ret) => ret,
            Err(errno) => -(errno as isize),
        };
        trap_frame[Register::a0] = ret as usize;
        Cpu::write_sepc(Cpu::read_sepc() + 4); // Skip the ecall instruction
        None
    };

    let mut cpu = Cpu::current();
    let scheduler = cpu.scheduler_mut();

    if let Some(ret) = ret {
        let trap_frame = scheduler.trap_frame_mut();
        trap_frame[Register::a0] = ret as usize;
        Cpu::write_sepc(Cpu::read_sepc() + 4); // Skip the ecall instruction
    }

    // In case our current process was set to waiting state we need to reschedule
    if scheduler.get_current_thread().lock().get_state() == ThreadState::Waiting {
        scheduler.schedule();
    }
}

fn handle_unhandled_exception() {
    let cause = InterruptCause::from_scause();
    let stval = Cpu::read_stval();
    let sepc = Cpu::read_sepc();
    let mut cpu = Cpu::current();
    let scheduler = cpu.scheduler_mut();
    let (message, from_userspace) = scheduler.get_current_process().with_lock(|p| {
        let from_userspace =
            p.get_page_table().is_userspace_address(sepc);
        (format!(
            "Unhandled exception!\nName: {}\nException code: {}\nstval: 0x{:x}\nsepc: 0x{:x}\nFrom Userspace: {}\nProcess name: {}\n{:?}",
            cause.get_reason(),
            cause.get_exception_code(),
            stval,
            sepc,
            from_userspace,
            p.get_name(),
            scheduler.trap_frame()
        ), from_userspace)
    });
    if from_userspace {
        info!("{}", message);
        scheduler.kill_current_process();
        return;
    }
    panic!("{}", message);
}

#[unsafe(no_mangle)]
extern "C" fn handle_exception() {
    let cause = InterruptCause::from_scause();
    match cause.get_exception_code() {
        ENVIRONMENT_CALL_FROM_U_MODE => handle_syscall(),
        _ => handle_unhandled_exception(),
    }
}

#[unsafe(no_mangle)]
extern "C" fn handle_unimplemented() {
    let sepc = Cpu::read_sepc();
    let cause = InterruptCause::from_scause();
    panic!(
        "Unimplemeneted trap occurred! (sepc: {:x?}) (cause: {:?})",
        sepc,
        cause.get_reason(),
    );
}
