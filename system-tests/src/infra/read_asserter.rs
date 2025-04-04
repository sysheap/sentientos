use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

use super::searchable_buffer::SearchableBuffer;

const DEFAULT_BUFFER_SIZE: usize = 1024;

pub struct ReadAsserter<Reader: AsyncRead + Unpin> {
    reader: Reader,
    buffer: SearchableBuffer,
    // It is important to only keep one stderr instance
    // Otherwise the output could be interlaved, especially with
    // write_all
    stderr: tokio::io::Stderr,
}

impl<Reader: AsyncRead + Unpin> ReadAsserter<Reader> {
    pub fn new(reader: Reader) -> Self {
        Self {
            reader,
            buffer: SearchableBuffer::new(Vec::with_capacity(DEFAULT_BUFFER_SIZE)),
            stderr: tokio::io::stderr(),
        }
    }

    pub async fn assert_read_until(&mut self, needle: &str) -> Vec<u8> {
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
            let input = &local_buffer[0..bytes];
            self.print_to_stderr(input).await;
            self.buffer.append(input);
        }
    }

    async fn print_to_stderr(&mut self, data: &[u8]) {
        self.stderr
            .write_all(data)
            .await
            .expect("Write to stderr must succeed.");
    }
}
