use crate::processes::{process::Pid, process_list};
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
        let mut notified = false;
        for pid in &self.wakeup_queue {
            if process_list::notify_input(*pid, byte) {
                notified = true;
            }
        }
        self.wakeup_queue.clear();
        if notified {
            return;
        }
        self.data.push_back(byte);
    }

    pub fn pop(&mut self) -> Option<u8> {
        self.data.pop_front()
    }
}
