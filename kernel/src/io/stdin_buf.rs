use crate::{
    cpu,
    processes::{
        process::{Pid, ProcessState},
        scheduler, timer,
    },
};
use alloc::collections::{BTreeSet, VecDeque};
use common::mutex::Mutex;

pub static STDIN_BUFFER: Mutex<StdinBuffer> = Mutex::new(StdinBuffer::new());

pub struct StdinBuffer {
    data: VecDeque<u8>,
    wakeup_queue: BTreeSet<Pid>,
}

impl StdinBuffer {
    const fn new() -> Self {
        StdinBuffer {
            data: VecDeque::new(),
            wakeup_queue: BTreeSet::new(),
        }
    }

    pub fn register_wakeup(&mut self, pid: Pid) {
        self.wakeup_queue.insert(pid);
    }

    pub fn push(&mut self, byte: u8) {
        let notified = !self.wakeup_queue.is_empty();
        scheduler::THE.with_lock(|s| {
            for pid in &self.wakeup_queue {
                if let Some(process) = s.get_process(*pid) {
                    process.with_lock(|mut p| {
                        p.set_state(ProcessState::Runnable);
                        p.set_syscall_return_code(byte as usize);
                    })
                }
            }
        });
        self.wakeup_queue.clear();
        if notified {
            if !cpu::is_timer_enabled() {
                // Enable timer because we were sleeping and waiting
                // for input
                timer::set_timer(0);
            }
            return;
        }
        self.data.push_back(byte);
    }

    pub fn pop(&mut self) -> Option<u8> {
        self.data.pop_front()
    }
}
