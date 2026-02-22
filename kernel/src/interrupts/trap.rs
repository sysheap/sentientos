use super::trap_cause::{InterruptCause, exception::ENVIRONMENT_CALL_FROM_U_MODE};
use crate::{
    cpu::Cpu,
    debug, info,
    interrupts::plic::{InterruptSource, PLIC},
    io::{stdin_buf::STDIN_BUFFER, uart::QEMU_UART},
    memory::VirtAddr,
    processes::{task::Task, thread::ThreadState, timer, waker::ThreadWaker},
    syscalls::linux::{LinuxSyscallHandler, LinuxSyscalls},
};
use common::syscalls::trap_frame::{Register, TrapFrame};
use core::{
    panic,
    task::{Context, Poll},
};

// SAFETY: Called from trap.S assembly; must use C ABI and fixed symbol name.
#[unsafe(no_mangle)]
extern "C" fn get_process_satp_value() -> usize {
    Cpu::with_current_process(|p| p.get_satp_value())
}

// SAFETY: Called from trap.S assembly; must use C ABI and fixed symbol name.
#[unsafe(no_mangle)]
extern "C" fn handle_timer_interrupt() {
    timer::wakeup_wakers();
    Cpu::with_scheduler(|mut s| s.schedule());
}

// SAFETY: Called from trap.S assembly; must use C ABI and fixed symbol name.
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

    let mut ctrl_c = false;
    let mut ctrl_d = false;
    {
        let uart = QEMU_UART.lock();
        while let Some(input) = uart.read() {
            match input {
                3 => ctrl_c = true,
                4 => ctrl_d = true,
                _ => STDIN_BUFFER.lock().push(input),
            }
        }
    }

    plic.complete_interrupt(plic_interrupt);
    drop(plic);

    if ctrl_c {
        Cpu::with_scheduler(|mut s| {
            s.send_ctrl_c();
        });
    }
    if ctrl_d {
        crate::debugging::dump_current_state();
    }
}

// Check if we still own the thread (syscall might have set it to Waiting or another CPU
// might have stolen it). If we don't own it, save state and reschedule. Returns true if
// we should continue executing on this CPU.
fn check_thread_ownership_and_reschedule_if_needed(trap_frame: TrapFrame) -> bool {
    Cpu::with_scheduler(|mut s| {
        let cpu_id = Cpu::cpu_id();
        let should_reschedule = s.get_current_thread().with_lock(|mut t| {
            match t.get_state() {
                ThreadState::Running {
                    cpu_id: running_cpu,
                } if running_cpu == cpu_id => {
                    // We still own the thread, continue on this CPU
                    false
                }
                ThreadState::Running { cpu_id: other_cpu } => {
                    // Another CPU stole this thread - indicates a race condition bug.
                    // The other CPU is running with stale register state.
                    panic!(
                        "Thread {} was stolen by CPU {} while CPU {} was still in syscall handler",
                        t.get_tid(),
                        other_cpu,
                        cpu_id
                    );
                }
                ThreadState::Waiting | ThreadState::Runnable => {
                    // Syscall put us in Waiting (and possibly got woken to Runnable).
                    // Save state before rescheduling.
                    let sepc = Cpu::read_sepc() + 4; // Skip ecall
                    t.set_register_state(trap_frame);
                    t.set_program_counter(VirtAddr::new(sepc));
                    true
                }
                ThreadState::Zombie(_) => {
                    // Thread was killed by another CPU while we were in a syscall.
                    // No need to save state — just reschedule.
                    true
                }
            }
        });

        if should_reschedule {
            s.schedule();
            false
        } else {
            true
        }
    })
}

fn handle_syscall() {
    let mut trap_frame = Cpu::read_trap_frame();

    let task_trap_frame = trap_frame.clone();
    let mut task = Task::new(async move {
        let mut handler = LinuxSyscallHandler::new();
        handler.handle(&task_trap_frame).await
    });
    let waker = ThreadWaker::new_waker(Cpu::current_thread_weak());
    let mut cx = Context::from_waker(&waker);
    if let Poll::Ready(result) = task.poll(&mut cx) {
        let replaced = Cpu::with_scheduler(|s| {
            s.get_current_thread().with_lock(|mut t| {
                let r = t.registers_replaced();
                if r {
                    t.set_registers_replaced(false);
                }
                r
            })
        });
        if replaced {
            Cpu::with_scheduler(|mut s| {
                if !s.set_cpu_reg_for_current_thread() {
                    s.schedule();
                }
            });
        } else {
            let ret = match result {
                Ok(ret) => ret,
                Err(errno) => -(errno as isize),
            };
            trap_frame[Register::a0] = ret.cast_unsigned();

            if check_thread_ownership_and_reschedule_if_needed(trap_frame.clone()) {
                Cpu::write_trap_frame(trap_frame);
                Cpu::write_sepc(Cpu::read_sepc() + 4); // Skip ecall
            }
        }
    } else {
        // Syscall pending - suspend and reschedule atomically.
        // We must hold the scheduler lock across suspend+schedule to prevent
        // another CPU from waking and stealing this thread before we reschedule.
        Cpu::with_scheduler(|mut s| {
            // Save register state BEFORE suspending.
            // When thread suspends, queue_current_process_back won't save registers
            // (since state is Waiting, not Running), so we must save them here.
            let sepc = Cpu::read_sepc();
            s.get_current_thread().with_lock(|mut t| {
                t.set_register_state(trap_frame);
                t.set_program_counter(VirtAddr::new(sepc));
                t.set_syscall_task_and_suspend(task);
            });
            s.schedule();
        });
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
            p.get_page_table().is_userspace_address(VirtAddr::new(sepc));
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
        scheduler.kill_current_process(0);
        return;
    }
    panic!("{}", message);
}

// SAFETY: Called from trap.S assembly; must use C ABI and fixed symbol name.
#[unsafe(no_mangle)]
extern "C" fn handle_exception() {
    let cause = InterruptCause::from_scause();
    match cause.get_exception_code() {
        ENVIRONMENT_CALL_FROM_U_MODE => handle_syscall(),
        _ => handle_unhandled_exception(),
    }
}

// SAFETY: Called from trap.S assembly; must use C ABI and fixed symbol name.
#[unsafe(no_mangle)]
extern "C" fn handle_supervisor_software_interrupt() {
    // This interrupt is fired when we kill a thread
    // It could be that our cpu is currently running this
    // thread, therefore, reschedule.
    Cpu::with_scheduler(|mut s| s.schedule());
    Cpu::clear_supervisor_software_interrupt();
}

// SAFETY: Called from trap.S assembly; must use C ABI and fixed symbol name.
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
