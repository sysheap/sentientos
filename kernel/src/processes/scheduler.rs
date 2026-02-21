use super::{
    process::{POWERSAVE_TID, ProcessRef},
    process_table::{self},
    thread::{ThreadRef, ThreadState},
};
use crate::{
    cpu::Cpu,
    debug, info,
    processes::{
        thread::{SyscallTask, Thread},
        timer,
        waker::ThreadWaker,
    },
    test::qemu_exit,
};
use alloc::sync::Arc;
use common::syscalls::trap_frame::Register;
use core::task::{Context, Poll};

pub struct CpuScheduler {
    current_thread: ThreadRef,
    powersave_thread: ThreadRef,
}

enum ProcessMode {
    KernelSyscallTask(SyscallTask),
    Userspace,
}

impl CpuScheduler {
    pub fn new() -> Self {
        let powersave_thread = Thread::create_powersave_thread();

        Self {
            current_thread: powersave_thread.clone(),
            powersave_thread,
        }
    }

    pub fn get_current_thread(&self) -> &ThreadRef {
        &self.current_thread
    }

    pub fn get_current_process(&self) -> ProcessRef {
        self.current_thread.lock().process()
    }

    pub fn is_current_process_energy_saver(&self) -> bool {
        Arc::ptr_eq(&self.current_thread, &self.powersave_thread)
    }

    pub fn schedule(&mut self) {
        debug!("Schedule next process");
        while let ProcessMode::KernelSyscallTask(task) = self.prepare_next_process() {
            debug!("Running syscall task");
            if self.run_syscall_task(task) {
                // we completed the syscall, lets give the process the chance to run directly
                break;
            }
        }

        debug!("Scheduling userspace process");
        if self.is_current_process_energy_saver() {
            timer::set_timer(50);
        } else {
            timer::set_timer(10);
        }
    }

    pub fn run_syscall_task(&mut self, mut task: SyscallTask) -> bool {
        let waker = ThreadWaker::new_waker(Arc::downgrade(&self.current_thread));
        let mut cx = Context::from_waker(&waker);
        if let Poll::Ready(result) = task.poll(&mut cx) {
            let replaced = self.current_thread.with_lock(|mut t| {
                let r = t.registers_replaced();
                if r {
                    t.set_registers_replaced(false);
                }
                r
            });
            if replaced {
                self.current_thread.lock().clear_wakeup_pending();
                self.set_cpu_reg_for_current_thread();
            } else {
                let ret = match result {
                    Ok(ret) => ret,
                    Err(errno) => -(errno as isize),
                };
                self.current_thread.with_lock(|mut t| {
                    t.clear_wakeup_pending();
                    let trap_frame = t.get_register_state_mut();
                    trap_frame[Register::a0] = ret.cast_unsigned();
                    let pc = t.get_program_counter();
                    t.set_program_counter(pc + 4); // Skip the ecall instruction
                });
                self.set_cpu_reg_for_current_thread();
            }
            true
        } else {
            // Use self.current_thread directly instead of Cpu::with_current_thread
            // to avoid trying to acquire scheduler lock while already holding it
            self.current_thread
                .lock()
                .set_syscall_task_and_suspend(task);
            false
        }
    }

    pub fn kill_current_process(&mut self, exit_status: i32) {
        let tid = self.current_thread.lock().process().with_lock(|p| {
            // TODO: Kill other threads first
            assert_eq!(
                p.threads_len(),
                1,
                "We currently don't support to kill other threads"
            );

            assert!(Arc::ptr_eq(&self.current_thread, &p.main_thread()));

            p.main_tid()
        });

        process_table::THE.lock().kill(tid, exit_status);
    }

    pub fn send_ctrl_c(&mut self) {
        process_table::THE.with_lock(|mut pt| {
            let highest_pid = pt.get_highest_tid_without(&["sesh"]);

            if let Some(pid) = highest_pid {
                pt.kill(pid, 0);
            }
        });
        self.schedule();
    }

    fn queue_current_process_back(&mut self) {
        if self.current_thread.lock().get_tid() == POWERSAVE_TID {
            debug!("Current thread is already powersave thread - don't queuing back");
            return;
        }
        let cpu_id = Cpu::cpu_id();
        self.swap_current_with_powersave().with_lock(|mut t| {
            // Only save state when preempting a Running thread on THIS CPU.
            // - Running on other CPU: another CPU stole this thread, don't touch it
            // - Waiting: state was already saved before thread suspended
            // - Runnable: state was saved, thread was woken by another CPU
            if t.get_state() == (ThreadState::Running { cpu_id }) {
                t.set_state(ThreadState::Runnable);
                t.set_program_counter(Cpu::read_sepc());
                t.set_register_state(Cpu::read_trap_frame());
            }
            debug!("Saved thread {} back", *t);
        });
    }

    fn prepare_next_process(&mut self) -> ProcessMode {
        self.queue_current_process_back();

        process_table::THE.with_lock(|mut pt| {
            if pt.is_empty() {
                info!("No more processes to schedule, shutting down system");
                qemu_exit::exit_success();
            }

            debug!("Getting next runnable process");

            assert!(
                self.is_current_process_energy_saver(),
                "Current thread must be powersave thread to not have any dangling references"
            );

            // next_runnable already sets the state to ThreadState::Running { cpu_id }
            if let Some(next) = pt.next_runnable() {
                debug!("Next runnable is {}", *next.lock());
                self.current_thread = next;
            } else {
                // No runnable threads, use powersave and mark it as running on this CPU
                self.powersave_thread.with_lock(|mut t| {
                    t.set_state(ThreadState::Running {
                        cpu_id: Cpu::cpu_id(),
                    });
                });
                debug!("Next runnable is powersave");
            }
        });

        let syscall_task = self.current_thread.with_lock(|mut t| t.take_syscall_task());

        if let Some(task) = syscall_task {
            ProcessMode::KernelSyscallTask(task)
        } else {
            self.set_cpu_reg_for_current_thread();
            ProcessMode::Userspace
        }
    }

    pub fn set_cpu_reg_for_current_thread(&self) {
        self.current_thread.with_lock(|t| {
            let cpu_id = Cpu::cpu_id();
            // Assert thread is running on this CPU - fail fast on corruption
            assert!(
                t.get_state() == ThreadState::Running { cpu_id },
                "Thread {} not assigned to this CPU (state: {:?}, expected cpu: {})",
                t.get_tid(),
                t.get_state(),
                cpu_id
            );

            let pc = t.get_program_counter();
            Cpu::write_trap_frame(t.get_register_state().clone());
            Cpu::write_sepc(pc);
            Cpu::set_ret_to_kernel_mode(t.get_in_kernel_mode());
        });
    }

    fn swap_current_with_powersave(&mut self) -> ThreadRef {
        core::mem::replace(&mut self.current_thread, self.powersave_thread.clone())
    }
}
