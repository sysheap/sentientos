use core::slice;

use crate::util::align_up;

pub struct ConsumableBuffer<'a> {
    buffer: &'a [u8],
    position: usize,
}

impl<'a> ConsumableBuffer<'a> {
    pub fn new(buffer: &'a [u8]) -> ConsumableBuffer<'a> {
        ConsumableBuffer {
            buffer,
            position: 0,
        }
    }

    pub fn consume_slice(&mut self, size: usize) -> Option<&'a [u8]> {
        if self.position + size > self.buffer.len() {
            return None;
        }

        if size == 0 {
            return Some(&[]);
        }

        let result = &self.buffer[self.position..self.position + size];
        self.position += size;
        Some(result)
    }

    pub fn consume_sized_type<T: FromU8Buffer>(&mut self) -> Option<T> {
        let size = core::mem::size_of::<T>();
        let result = self.consume_slice(size)?;
        Some(T::from_u8_buffer(result))
    }

    pub fn consume_alignment(&mut self, alignment: usize) -> Option<()> {
        let aligned_value = align_up(self.position, alignment);
        let diff = aligned_value - self.position;
        self.consume_slice(diff)?;
        Some(())
    }

    pub fn consume_str(&mut self) -> Option<&'a str> {
        let mut length = 0;
        while self.position + length < self.buffer.len() && self.buffer[self.position + length] != 0
        {
            length += 1;
        }
        // Check if we really found a null-terminated string
        if self.buffer[self.position + length] != 0 {
            return None;
        }

        let string = unsafe {
            core::str::from_utf8(slice::from_raw_parts(&self.buffer[self.position], length)).ok()?
        };

        // Consume null byte
        length += 1;

        self.position += length;

        Some(string)
    }

    pub fn empty(&self) -> bool {
        self.position >= self.buffer.len()
    }

    pub fn size_left(&self) -> usize {
        if self.position >= self.buffer.len() {
            0
        } else {
            self.buffer.len() - self.position
        }
    }
}

pub trait FromU8Buffer: Copy {
    fn from_u8_buffer(buffer: &[u8]) -> Self;
}
