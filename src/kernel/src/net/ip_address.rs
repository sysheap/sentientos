use core::fmt::Display;

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct IpV4Address([u8; 4]);

impl IpV4Address {
    pub const fn new(octet1: u8, octet2: u8, octet3: u8, octet4: u8) -> Self {
        Self([octet1, octet2, octet3, octet4])
    }

    pub fn as_big_endian_u32(&self) -> u32 {
        ((self.0[0] as u32) << 24)
            + ((self.0[1] as u32) << 16)
            + ((self.0[2] as u32) << 8)
            + ((self.0[3] as u32) << 0)
    }
}

impl Display for IpV4Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}
