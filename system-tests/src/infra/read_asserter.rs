use std::time::Duration;

use tokio::{
    io::{AsyncRead, AsyncReadExt},
    time::timeout,
};

use super::searchable_buffer::SearchableBuffer;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_BUFFER_SIZE: usize = 1024;

pub struct ReadAsserter<Reader: AsyncRead + Unpin> {
    reader: Reader,
    buffer: SearchableBuffer,
    timeout: Duration,
}

impl<Reader: AsyncRead + Unpin> ReadAsserter<Reader> {
    pub fn new(reader: Reader) -> Self {
        Self {
            reader,
            buffer: SearchableBuffer::new(Vec::with_capacity(DEFAULT_BUFFER_SIZE)),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub async fn assert_read_until(&mut self, needle: &str) -> Vec<u8> {
        let result = timeout(self.timeout.clone(), self.read_until(needle)).await;

        if let Ok(result) = result {
            return result;
        } else {
            panic!("Expected\n{needle}\nFound\n{}", self.buffer.as_str());
        }
    }

    async fn read_until(&mut self, needle: &str) -> Vec<u8> {
        loop {
            if let Some(front) = self.buffer.find_and_remove(needle) {
                return front;
            }
            let mut local_buffer = [0u8; 1024];
            let bytes = self
                .reader
                .read(&mut local_buffer)
                .await
                .expect("Read must succeed.");
            self.buffer.append(&local_buffer[0..bytes]);
        }
    }
}
