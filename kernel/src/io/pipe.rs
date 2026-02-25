use crate::klibc::Spinlock;
use alloc::{collections::VecDeque, sync::Arc, vec::Vec};
use core::{
    cmp::min,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use headers::errno::Errno;

pub type SharedPipeBuffer = Arc<Spinlock<PipeInner>>;

pub struct PipeInner {
    data: VecDeque<u8>,
    read_wakers: Vec<Waker>,
    write_closed: bool,
    read_closed: bool,
}

impl PipeInner {
    fn new() -> Self {
        Self {
            data: VecDeque::new(),
            read_wakers: Vec::new(),
            write_closed: false,
            read_closed: false,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize, Errno> {
        if self.read_closed {
            return Err(Errno::EPIPE);
        }
        self.data.extend(data);
        self.wake_readers();
        Ok(data.len())
    }

    pub fn try_read(&mut self, count: usize) -> Result<Vec<u8>, Errno> {
        if !self.data.is_empty() {
            let actual = min(self.data.len(), count);
            return Ok(self.data.drain(..actual).collect());
        }
        if self.write_closed {
            return Ok(Vec::new());
        }
        Err(Errno::EAGAIN)
    }

    pub fn close_write(&mut self) {
        self.write_closed = true;
        self.wake_readers();
    }

    pub fn close_read(&mut self) {
        self.read_closed = true;
    }

    fn wake_readers(&mut self) {
        while let Some(waker) = self.read_wakers.pop() {
            waker.wake();
        }
    }
}

pub fn new_pipe() -> SharedPipeBuffer {
    Arc::new(Spinlock::new(PipeInner::new()))
}

pub struct ReadPipe {
    buffer: SharedPipeBuffer,
    max_count: usize,
    is_registered: bool,
}

impl ReadPipe {
    pub fn new(buffer: SharedPipeBuffer, max_count: usize) -> Self {
        Self {
            buffer,
            max_count,
            is_registered: false,
        }
    }
}

impl Future for ReadPipe {
    type Output = Vec<u8>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let buffer = self.buffer.clone();
        let max_count = self.max_count;
        let mut pipe = buffer.lock();
        if !pipe.data.is_empty() {
            let actual = min(pipe.data.len(), max_count);
            return Poll::Ready(pipe.data.drain(..actual).collect());
        }
        if pipe.write_closed {
            return Poll::Ready(Vec::new());
        }
        if !self.is_registered {
            pipe.read_wakers.push(cx.waker().clone());
            self.is_registered = true;
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::PipeInner;
    use headers::errno::Errno;

    #[test_case]
    fn write_then_read() {
        let mut pipe = PipeInner::new();
        assert_eq!(pipe.write(b"hello").expect("write"), 5);
        assert_eq!(pipe.try_read(10).expect("read"), b"hello");
    }

    #[test_case]
    fn read_empty_returns_eagain() {
        let mut pipe = PipeInner::new();
        assert_eq!(pipe.try_read(10), Err(Errno::EAGAIN));
    }

    #[test_case]
    fn read_after_write_close_returns_eof() {
        let mut pipe = PipeInner::new();
        pipe.close_write();
        let data = pipe.try_read(10).expect("read after close");
        assert!(data.is_empty());
    }

    #[test_case]
    fn write_after_read_close_returns_epipe() {
        let mut pipe = PipeInner::new();
        pipe.close_read();
        assert_eq!(pipe.write(b"hello"), Err(Errno::EPIPE));
    }

    #[test_case]
    fn partial_read() {
        let mut pipe = PipeInner::new();
        pipe.write(b"hello world").expect("write");
        assert_eq!(pipe.try_read(5).expect("read1"), b"hello");
        assert_eq!(pipe.try_read(10).expect("read2"), b" world");
    }

    #[test_case]
    fn data_available_after_write_close_is_readable() {
        let mut pipe = PipeInner::new();
        pipe.write(b"data").expect("write");
        pipe.close_write();
        assert_eq!(pipe.try_read(10).expect("read1"), b"data");
        let eof = pipe.try_read(10).expect("read2");
        assert!(eof.is_empty());
    }
}
