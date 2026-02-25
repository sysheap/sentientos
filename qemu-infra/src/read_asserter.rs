use std::{fmt, time::Duration};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

use crate::searchable_buffer::SearchableBuffer;

const DEFAULT_BUFFER_SIZE: usize = 1024;

#[derive(Debug)]
pub enum ReadError {
    Timeout {
        needle: String,
        buffered_output: String,
    },
    Eof {
        needle: String,
        buffered_output: String,
    },
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Timeout {
                needle,
                buffered_output,
            } => write!(
                f,
                "Timeout waiting for {needle:?}\nBuffered output:\n{buffered_output}"
            ),
            ReadError::Eof {
                needle,
                buffered_output,
            } => write!(
                f,
                "EOF while waiting for {needle:?}\nBuffered output:\n{buffered_output}"
            ),
        }
    }
}

impl std::error::Error for ReadError {}

pub struct ReadAsserter<Reader: AsyncRead + Unpin> {
    reader: Reader,
    buffer: SearchableBuffer,
    // It is important to only keep one stderr instance
    // Otherwise the output could be interleaved, especially with
    // write_all
    stderr: tokio::io::Stderr,
    timeout: Duration,
}

impl<Reader: AsyncRead + Unpin> ReadAsserter<Reader> {
    pub fn new(reader: Reader) -> Self {
        Self {
            reader,
            buffer: SearchableBuffer::new(Vec::with_capacity(DEFAULT_BUFFER_SIZE)),
            stderr: tokio::io::stderr(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn assert_read_until(&mut self, needle: &str) -> Result<Vec<u8>, ReadError> {
        let timeout = self.timeout;
        match tokio::time::timeout(timeout, self.read_until_inner(needle)).await {
            Ok(result) => result,
            Err(_) => {
                let buffered = String::from_utf8_lossy(self.buffer.peek()).into_owned();
                Err(ReadError::Timeout {
                    needle: needle.to_string(),
                    buffered_output: buffered,
                })
            }
        }
    }

    async fn read_until_inner(&mut self, needle: &str) -> Result<Vec<u8>, ReadError> {
        loop {
            if let Some(front) = self.buffer.find_and_remove(needle) {
                return Ok(front);
            }
            let mut local_buffer = [0u8; 1024];
            let bytes = self
                .reader
                .read(&mut local_buffer)
                .await
                .expect("Read must succeed.");
            if bytes == 0 {
                let buffered = String::from_utf8_lossy(self.buffer.peek()).into_owned();
                return Err(ReadError::Eof {
                    needle: needle.to_string(),
                    buffered_output: buffered,
                });
            }
            let input = &local_buffer[0..bytes];
            self.print_to_stderr(input).await;
            self.buffer.append(input);
        }
    }

    pub async fn read_available(&mut self) -> Vec<u8> {
        let timeout = Duration::from_millis(100);
        loop {
            let mut local_buffer = [0u8; 1024];
            match tokio::time::timeout(timeout, self.reader.read(&mut local_buffer)).await {
                Ok(Ok(bytes)) if bytes > 0 => {
                    let input = &local_buffer[0..bytes];
                    self.print_to_stderr(input).await;
                    self.buffer.append(input);
                }
                _ => break,
            }
        }
        self.buffer.drain()
    }

    async fn print_to_stderr(&mut self, data: &[u8]) {
        self.stderr
            .write_all(data)
            .await
            .expect("Write to stderr must succeed.");
    }
}
