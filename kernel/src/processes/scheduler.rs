use super::{
    process::{POWERSAVE_TID, ProcessRef},
    process_table::{self},
    thread::{ThreadRef, ThreadState},
};
use crate::{
    cpu::Cpu,
    debug, info,
    memory::VirtAddr,
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

    // Resumes a previously-suspended async syscall task. Returns true if the
    // syscall completed and the thread is ready to return to userspace, false
    // if it yielded again (Pending) or the thread was killed.
    pub fn run_syscall_task(&mut self, mut task: SyscallTask) -> bool {
        let waker = ThreadWaker::new_waker(Arc::downgrade(&self.current_thread));
        let mut cx = Context::from_waker(&waker);
        if let Poll::Ready(result) = task.poll(&mut cx) {
            // Same dual return path as handle_syscall: if execve replaced the
            // registers, skip the normal a0/PC return logic.
            let replaced = self.current_thread.with_lock(|mut t| {
                let r = t.registers_replaced();
                if r {
                    t.set_registers_replaced(false);
                }
                r
            });
            if replaced {
                self.current_thread.lock().clear_wakeup_pending();
                if !self.set_cpu_reg_for_current_thread() {
                    return false;
                }
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
                if !self.set_cpu_reg_for_current_thread() {
                    return false;
                }
            }
            true
        } else {
            // Still pending — store the task back and keep the thread waiting.
            self.current_thread
                .lock()
                .set_syscall_task_and_suspend(task);
            false
        }
    }

    pub fn kill_current_thread(&mut self, exit_status: i32) {
        let tid = self.current_thread.lock().get_tid();
        process_table::THE.lock().kill(tid, exit_status);
    }

    pub fn kill_current_process(&mut self, exit_status: i32) {
        let all_tids = self.current_thread.lock().process().lock().thread_tids();
        let mut pt = process_table::THE.lock();
        for tid in all_tids {
            pt.kill(tid, exit_status);
        }
    }

    pub fn send_ctrl_c(&mut self) {
        process_table::THE.with_lock(|mut pt| {
            let highest_pid = pt.get_highest_tid_without(&["sosh"]);

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
        let old = self.swap_current_with_powersave();
        let should_requeue = old.with_lock(|mut t| {
            if t.get_state() == (ThreadState::Running { cpu_id }) {
                t.set_state(ThreadState::Runnable);
                t.set_program_counter(VirtAddr::new(Cpu::read_sepc()));
                t.set_register_state(Cpu::read_trap_frame());
                debug!("Saved thread {} back", *t);
                true
            } else {
                false
            }
        });
        if should_requeue {
            process_table::RUN_QUEUE.lock().push_back(old);
        }
    }

    fn prepare_next_process(&mut self) -> ProcessMode {
        loop {
            self.queue_current_process_back();

            if process_table::is_empty() {
                info!("No more processes to schedule, shutting down system");
                qemu_exit::exit_success();
            }

            debug!("Getting next runnable process");

            assert!(
                self.is_current_process_energy_saver(),
                "Current thread must be powersave thread to not have any dangling references"
            );

            let next = process_table::RUN_QUEUE.lock().pop_front();
            if let Some(candidate) = next {
                let accepted = candidate.with_lock(|mut t| {
                    if t.get_state() == ThreadState::Runnable {
                        t.set_state(ThreadState::Running {
                            cpu_id: Cpu::cpu_id(),
                        });
                        true
                    } else {
                        false
                    }
                });
                if accepted {
                    debug!("Next runnable is {}", *candidate.lock());
                    self.current_thread = candidate;
                } else {
                    // Stale entry (killed/waiting since enqueued), discard and retry
                    continue;
                }
            } else {
                self.powersave_thread.with_lock(|mut t| {
                    t.set_state(ThreadState::Running {
                        cpu_id: Cpu::cpu_id(),
                    });
                });
                debug!("Next runnable is powersave");
            }

            // Acquire the thread lock once for both task check and register load,
            // eliminating the gap where a thread could be killed between the two.
            let result = self.current_thread.with_lock(|mut t| {
                if let Some(task) = t.take_syscall_task() {
                    return Some(ProcessMode::KernelSyscallTask(task));
                }
                if matches!(t.get_state(), ThreadState::Zombie(_)) {
                    return None;
                }
                let cpu_id = Cpu::cpu_id();
                assert!(
                    t.get_state() == ThreadState::Running { cpu_id },
                    "Thread {} not assigned to this CPU (state: {:?}, expected cpu: {})",
                    t.get_tid(),
                    t.get_state(),
                    cpu_id
                );
                let pc = t.get_program_counter();
                Cpu::write_trap_frame(t.get_register_state().clone());
                Cpu::write_sepc(pc.as_usize());
                Cpu::set_ret_to_kernel_mode(t.get_in_kernel_mode());
                Some(ProcessMode::Userspace)
            });
            if let Some(mode) = result {
                return mode;
            }
            // Thread was killed — retry
        }
    }

    pub fn set_cpu_reg_for_current_thread(&self) -> bool {
        self.current_thread.with_lock(|t| {
            let cpu_id = Cpu::cpu_id();
            if matches!(t.get_state(), ThreadState::Zombie(_)) {
                debug!(
                    "Thread {} was killed during scheduling, rescheduling",
                    t.get_tid()
                );
                return false;
            }
            assert!(
                t.get_state() == ThreadState::Running { cpu_id },
                "Thread {} not assigned to this CPU (state: {:?}, expected cpu: {})",
                t.get_tid(),
                t.get_state(),
                cpu_id
            );

            let pc = t.get_program_counter();
            Cpu::write_trap_frame(t.get_register_state().clone());
            Cpu::write_sepc(pc.as_usize());
            Cpu::set_ret_to_kernel_mode(t.get_in_kernel_mode());
            true
        })
    }

    fn swap_current_with_powersave(&mut self) -> ThreadRef {
        core::mem::replace(&mut self.current_thread, self.powersave_thread.clone())
    }
}
