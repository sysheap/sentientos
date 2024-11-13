use alloc::vec::Vec;
use common::mutex::Mutex;

pub static STDIN_BUFFER: Mutex<StdinBuffer> = Mutex::new(StdinBuffer::new());

pub struct StdinBuffer {
    data: Vec<u8>,
}

impl StdinBuffer {
    const fn new() -> Self {
        StdinBuffer { data: Vec::new() }
    }

    pub fn push(&mut self, byte: u8) {
        self.data.push(byte);
    }

    pub fn pop(&mut self) -> Option<u8> {
        self.data.pop()
    }
}
