use core::ops::{Deref, DerefMut};

use crate::{debug, processes::process_table};
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
        self.data.push_back(byte);

        debug!("Waking up following tids={:?}", self.wakeup_queue);
        process_table::THE.with_lock(|pt| {
            while let Some(tid) = &self.wakeup_queue.pop_first() {
                if let Some(thread) = pt.get_thread(*tid) {
                    thread.with_lock(|mut t| {
                        t.resume_on_syscall_linux();
                        debug!("Resume on syscall set on thread={}", *t);
                    })
                }
            }
        });
    }
}

impl Deref for StdinBuffer {
    type Target = VecDeque<u8>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for StdinBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
