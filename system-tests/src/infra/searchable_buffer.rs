pub(super) struct SearchableBuffer {
    buffer: Vec<u8>,
}

impl SearchableBuffer {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self { buffer }
    }

    pub fn find_and_remove(&mut self, needle: &str) -> Option<Vec<u8>> {
        if let Some(index) = self.find_end_index_in_buffer(needle) {
            let front = self.shift_to_front(index);
            return Some(front);
        }
        None
    }

    pub fn append(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    fn find_end_index_in_buffer(&self, needle: &str) -> Option<usize> {
        let needle = needle.as_bytes();
        self.buffer
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|start_index| start_index + needle.len())
    }

    fn shift_to_front(&mut self, index: usize) -> Vec<u8> {
        let mut part = self.buffer.split_off(index);
        std::mem::swap(&mut part, &mut self.buffer);
        part
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUT: &[u8] = "foo 42 bar".as_bytes();

    #[test]
    fn end_index() {
        let input = INPUT.to_vec();

        let searchable_buffer = SearchableBuffer::new(input);

        assert_eq!(searchable_buffer.find_end_index_in_buffer("42"), Some(6));
    }

    #[test]
    fn shift() {
        let input = INPUT.to_vec();

        let mut searchable_buffer = SearchableBuffer::new(input);

        let front = searchable_buffer.shift_to_front(6);

        assert_eq!(&front, "foo 42".as_bytes());
        assert_eq!(searchable_buffer.buffer, " bar".as_bytes());
    }
}
