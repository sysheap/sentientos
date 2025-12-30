use crate::{debug, klibc::Spinlock, processes::process_table};
use alloc::{
    collections::{BTreeSet, VecDeque},
    vec::Vec,
};
use common::pid::Tid;
use core::cmp::min;

pub static STDIN_BUFFER: Spinlock<StdinBuffer> = Spinlock::new(StdinBuffer::new());

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

        if self.wakeup_queue.is_empty() {
            return;
        }

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

    pub fn pop(&mut self) -> Option<u8> {
        self.data.pop_front()
    }

    pub fn get(&mut self, count: usize) -> Vec<u8> {
        let actual_count = min(self.data.len(), count);

        self.data.drain(..actual_count).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::io::stdin_buf::StdinBuffer;

    #[test_case]
    fn empty() {
        let stdin = StdinBuffer::new();
        assert!(stdin.is_empty());
    }

    #[test_case]
    fn order_is_correct() {
        let mut stdin = StdinBuffer::new();

        stdin.push(42);
        stdin.push(43);
        stdin.push(44);

        assert_eq!(stdin.pop(), Some(42));
        assert_eq!(stdin.pop(), Some(43));
        assert_eq!(stdin.pop(), Some(44));
    }

    #[test_case]
    fn get_works() {
        let mut stdin = StdinBuffer::new();

        stdin.push(42);
        stdin.push(43);
        stdin.push(44);

        assert_eq!(stdin.get(1), &[42]);
        assert_eq!(stdin.get(10), &[43, 44]);
        assert_eq!(stdin.get(10), &[]);
    }
}
