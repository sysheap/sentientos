use crate::{
    cpu::Cpu,
    debug,
    processes::{process_table, timer},
};
use alloc::collections::{BTreeSet, VecDeque};
use common::{mutex::Mutex, pid::Tid};

pub static STDIN_BUFFER: Mutex<StdinBuffer> = Mutex::new(StdinBuffer::new());

pub struct StdinBuffer {
    data: VecDeque<u8>,
    wakeup_queue: BTreeSet<Tid>,
}

impl StdinBuffer {
    const fn new() -> Self {
        StdinBuffer {
            data: VecDeque::new(),
            wakeup_queue: BTreeSet::new(),
        }
    }

    pub fn register_wakeup(&mut self, tid: Tid) {
        self.wakeup_queue.insert(tid);
    }

    pub fn push(&mut self, byte: u8) {
        let notified = !self.wakeup_queue.is_empty();
        debug!("Waking up following tids={:?}", self.wakeup_queue);
        process_table::THE.with_lock(|pt| {
            for tid in &self.wakeup_queue {
                if let Some(thread) = pt.get_thread(*tid) {
                    thread.with_lock(|mut t| {
                        debug!("Resume on syscall set on thread={}", *t);
                        t.resume_on_syscall(byte);
                    })
                }
            }
        });
        Cpu::with_scheduler(|s| {
            if notified && s.is_current_process_energy_saver() {
                debug!("notified process and current process is energy saver");
                s.schedule();
            }
        });
        self.wakeup_queue.clear();
        if notified {
            if !Cpu::is_timer_enabled() {
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
