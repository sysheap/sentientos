#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct UDPDescriptor(u64);

impl UDPDescriptor {
    pub const fn new(fd: u64) -> Self {
        Self(fd)
    }

    pub fn get(&self) -> u64 {
        self.0
    }
}
