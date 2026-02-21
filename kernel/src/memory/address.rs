use core::{
    fmt,
    ops::{Add, AddAssign, Sub},
};

/// Physical memory address (zero-cost wrapper around usize)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(usize);

/// Virtual memory address (zero-cost wrapper around usize)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(usize);

impl PhysAddr {
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    #[allow(dead_code)]
    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }

    #[allow(dead_code)]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    #[allow(dead_code)]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    #[allow(dead_code)]
    pub const fn from_page_number(ppn: usize) -> Self {
        Self(ppn << 12)
    }

    #[allow(dead_code)]
    pub const fn page_number(self) -> usize {
        self.0 >> 12
    }

    pub const fn is_page_aligned(self) -> bool {
        self.0 & 0xFFF == 0
    }

    #[allow(dead_code)]
    pub const fn align_down(self) -> Self {
        Self(self.0 & !0xFFF)
    }

    #[allow(dead_code)]
    pub const fn align_up(self) -> Self {
        Self((self.0 + 0xFFF) & !0xFFF)
    }
}

impl VirtAddr {
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    #[allow(dead_code)]
    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }

    #[allow(dead_code)]
    pub const fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    #[allow(dead_code)]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    #[allow(dead_code)]
    pub const fn from_page_number(vpn: usize) -> Self {
        Self(vpn << 12)
    }

    #[allow(dead_code)]
    pub const fn page_number(self) -> usize {
        self.0 >> 12
    }

    pub const fn is_page_aligned(self) -> bool {
        self.0 & 0xFFF == 0
    }

    #[allow(dead_code)]
    pub const fn align_down(self) -> Self {
        Self(self.0 & !0xFFF)
    }

    #[allow(dead_code)]
    pub const fn align_up(self) -> Self {
        Self((self.0 + 0xFFF) & !0xFFF)
    }

    /// Sv39 VPN index for page table level 0, 1, or 2.
    #[allow(dead_code)]
    pub const fn vpn_level(self, level: u8) -> usize {
        assert!(level < 3);
        (self.0 >> (12 + level as usize * 9)) & 0x1FF
    }

    #[allow(dead_code)]
    pub const fn page_offset(self) -> usize {
        self.0 & 0xFFF
    }
}

impl Add<usize> for PhysAddr {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for PhysAddr {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub<usize> for PhysAddr {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self {
        Self(self.0 - rhs)
    }
}

impl Sub for PhysAddr {
    type Output = usize;
    fn sub(self, rhs: Self) -> usize {
        self.0 - rhs.0
    }
}

impl Add<usize> for VirtAddr {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for VirtAddr {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub<usize> for VirtAddr {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self {
        Self(self.0 - rhs)
    }
}

impl Sub for VirtAddr {
    type Output = usize;
    fn sub(self, rhs: Self) -> usize {
        self.0 - rhs.0
    }
}

impl fmt::Display for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0)
    }
}

impl fmt::Display for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_phys_addr_basic() {
        let addr = PhysAddr::new(0x8000_0000);
        assert_eq!(addr.as_usize(), 0x8000_0000);
        assert_eq!(PhysAddr::zero().as_usize(), 0);
    }

    #[test_case]
    fn test_virt_addr_basic() {
        let addr = VirtAddr::new(0x1000);
        assert_eq!(addr.as_usize(), 0x1000);
        assert_eq!(VirtAddr::zero().as_usize(), 0);
    }

    #[test_case]
    fn test_page_number_conversion() {
        let addr = PhysAddr::from_page_number(0x8000);
        assert_eq!(addr.as_usize(), 0x8000 << 12);
        assert_eq!(addr.page_number(), 0x8000);

        let vaddr = VirtAddr::from_page_number(0x1000);
        assert_eq!(vaddr.as_usize(), 0x1000 << 12);
        assert_eq!(vaddr.page_number(), 0x1000);
    }

    #[test_case]
    fn test_alignment() {
        let addr = PhysAddr::new(0x8000_1234);
        assert!(!addr.is_page_aligned());
        assert_eq!(addr.align_down().as_usize(), 0x8000_1000);
        assert_eq!(addr.align_up().as_usize(), 0x8000_2000);

        let aligned = PhysAddr::new(0x8000_0000);
        assert!(aligned.is_page_aligned());
        assert_eq!(aligned.align_down().as_usize(), 0x8000_0000);
        assert_eq!(aligned.align_up().as_usize(), 0x8000_0000);
    }

    #[test_case]
    fn test_arithmetic() {
        let addr = PhysAddr::new(0x8000_0000);
        assert_eq!((addr + 0x1000).as_usize(), 0x8000_1000);
        assert_eq!((addr - 0x1000).as_usize(), 0x7FFF_F000);

        let other = PhysAddr::new(0x8000_2000);
        assert_eq!(other - addr, 0x2000);
    }

    #[test_case]
    fn test_vpn_level() {
        // All 9-bit VPN fields set (bits 38..0 all ones)
        let addr = VirtAddr::new(0x0000_007F_FFFF_FFFF);
        assert_eq!(addr.vpn_level(2), 0x1FF);
        assert_eq!(addr.vpn_level(1), 0x1FF);
        assert_eq!(addr.vpn_level(0), 0x1FF);

        let addr2 = VirtAddr::new(0x0000_0000_0040_1000);
        assert_eq!(addr2.vpn_level(2), 0);
        assert_eq!(addr2.vpn_level(1), 2);
        assert_eq!(addr2.vpn_level(0), 1);
    }

    #[test_case]
    fn test_page_offset() {
        let addr = VirtAddr::new(0x1234);
        assert_eq!(addr.page_offset(), 0x234);

        let aligned = VirtAddr::new(0x1000);
        assert_eq!(aligned.page_offset(), 0);
    }

    #[test_case]
    fn test_ordering() {
        let a1 = PhysAddr::new(0x1000);
        let a2 = PhysAddr::new(0x2000);
        let a3 = PhysAddr::new(0x1000);
        assert!(a1 < a2);
        assert_eq!(a1, a3);

        let v1 = VirtAddr::new(0x1000);
        let v2 = VirtAddr::new(0x2000);
        let v3 = VirtAddr::new(0x1000);
        assert!(v1 < v2);
        assert_eq!(v1, v3);
    }
}
