use crate::klibc::Spinlock;
use alloc::{collections::VecDeque, vec::Vec};
use core::{
    cmp::min,
    pin::Pin,
    task::{Context, Poll, Waker},
};

pub static STDIN_BUFFER: Spinlock<StdinBuffer> = Spinlock::new(StdinBuffer::new());

pub struct StdinBuffer {
    data: VecDeque<u8>,
    wakeup_queue: Vec<Waker>,
}

impl StdinBuffer {
    const fn new() -> Self {
        StdinBuffer {
            data: VecDeque::new(),
            wakeup_queue: Vec::new(),
        }
    }

    fn register_wakeup(&mut self, waker: Waker) {
        self.wakeup_queue.push(waker);
    }

    pub fn push(&mut self, byte: u8) {
        self.data.push_back(byte);
        while let Some(waker) = self.wakeup_queue.pop() {
            waker.wake();
        }
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

pub struct ReadStdin {
    max_count: usize,
    is_registered: bool,
}

impl ReadStdin {
    pub fn new(max_count: usize) -> Self {
        Self {
            max_count,
            is_registered: false,
        }
    }
}

impl Future for ReadStdin {
    type Output = Vec<u8>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut stdin = STDIN_BUFFER.lock();
        if !stdin.is_empty() {
            return Poll::Ready(stdin.get(self.max_count));
        }
        if !self.is_registered {
            let waker = cx.waker().clone();
            stdin.register_wakeup(waker);
            self.is_registered = true;
        }
        Poll::Pending
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
