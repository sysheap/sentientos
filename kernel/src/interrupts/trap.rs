use super::trap_cause::{InterruptCause, exception::ENVIRONMENT_CALL_FROM_U_MODE};
use crate::{
    cpu::Cpu,
    debug, info,
    interrupts::plic::{InterruptSource, PLIC},
    io::{stdin_buf::STDIN_BUFFER, uart::QEMU_UART},
    processes::{task::Task, thread::ThreadState, waker::TaskWaker},
    syscalls::{
        self,
        linux::{LinuxSyscallHandler, LinuxSyscalls},
    },
};
use common::syscalls::trap_frame::Register;
use core::{panic, task::Context};

#[unsafe(no_mangle)]
extern "C" fn get_process_satp_value() -> usize {
    Cpu::with_current_process(|p| p.get_page_table().get_satp_value_from_page_tables())
}

#[unsafe(no_mangle)]
extern "C" fn handle_timer_interrupt() {
    Cpu::with_scheduler(|mut s| s.schedule());
}

#[unsafe(no_mangle)]
fn handle_external_interrupt() {
    debug!("External interrupt occurred!");
    let mut plic = PLIC.lock();
    let plic_interrupt = match plic.get_next_pending() {
        Some(i) => i,
        None => return,
    };
    assert!(
        plic_interrupt == InterruptSource::Uart,
        "Plic interrupt should be uart."
    );

    let uart = QEMU_UART.lock();
    while let Some(input) = uart.read() {
        match input {
            3 => {
                Cpu::with_scheduler(|mut s| {
                    s.send_ctrl_c();
                });
            }
            4 => crate::debugging::dump_current_state(),
            _ => {
                STDIN_BUFFER.lock().push(input);
            }
        }
    }
    drop(uart);

    plic.complete_interrupt(plic_interrupt);

    drop(plic);
}

fn handle_syscall() {
    let mut trap_frame = Cpu::read_trap_frame();
    let nr = trap_frame[Register::a7];
    let arg = trap_frame[Register::a1];
    let ret = trap_frame[Register::a2];

    if (1 << 63) & nr > 0 {
        // We might need to get the current cpu again in handle_syscall
        if let Some(ret) = syscalls::handle_syscall(nr, arg, ret) {
            trap_frame[Register::a0] = ret as usize;
            Cpu::write_trap_frame(trap_frame);
            Cpu::write_sepc(Cpu::read_sepc() + 4); // Skip the ecall instruction
        }
    } else {
        let waker = TaskWaker::new();
        let mut cx = Context::from_waker(&waker);
        let mut task = Task::new(async move {
            let mut handler = LinuxSyscallHandler::new();
            handler.handle(&trap_frame).await
        });
        let result = match task.poll(&mut cx) {
            core::task::Poll::Ready(result) => result,
            core::task::Poll::Pending => panic!("Task is pending"),
        };
        let ret = match result {
            Ok(ret) => ret,
            Err(errno) => -(errno as isize),
        };
        trap_frame[Register::a0] = ret as usize;
        Cpu::write_trap_frame(trap_frame);
        Cpu::write_sepc(Cpu::read_sepc() + 4); // Skip the ecall instruction
    }

    let cpu = Cpu::current();

    let mut scheduler = cpu.scheduler().lock();
    // In case our current process was set to waiting state we need to reschedule
    if scheduler.get_current_thread().lock().get_state() == ThreadState::Waiting {
        scheduler.schedule();
    }
}

fn handle_unhandled_exception() {
    let cause = InterruptCause::from_scause();
    let stval = Cpu::read_stval();
    let sepc = Cpu::read_sepc();
    let cpu = Cpu::current();
    let mut scheduler = cpu.scheduler().lock();
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
            Cpu::read_trap_frame()
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
extern "C" fn handle_supervisor_software_interrupt() {
    // This interrupt is fired when we kill a thread
    // It could be that our cpu is currently running this
    // thread, therefore, reschedule.
    Cpu::with_scheduler(|mut s| s.schedule());
    Cpu::clear_supervisor_software_interrupt();
}

#[unsafe(no_mangle)]
extern "C" fn handle_unimplemented() {
    let sepc = Cpu::read_sepc();
    let cause = InterruptCause::from_scause();
    panic!(
        "Unimplemented trap occurred! (sepc: {:x?}) (cause: {:?})",
        sepc,
        cause.get_reason(),
    );
}
