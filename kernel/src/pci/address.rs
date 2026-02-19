use crate::memory::address::PhysAddr;
use core::fmt;

/// PCI address space (device-side view)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct PciAddr(usize);

/// PCI address as seen from CPU (after translation)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct PciCpuAddr(usize);

#[allow(dead_code)]
impl PciAddr {
    #[inline]
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0
    }

    /// Apply device-tree offset to translate from PCI to CPU address space.
    #[inline]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub const fn to_cpu_addr(self, offset: i64) -> PciCpuAddr {
        PciCpuAddr((self.0 as i64 + offset) as usize)
    }
}

#[allow(dead_code)]
impl PciCpuAddr {
    #[inline]
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0
    }

    /// CPU-visible PCI addresses are identity-mapped to physical addresses.
    #[inline]
    pub const fn as_phys_addr(self) -> PhysAddr {
        PhysAddr::new(self.0)
    }
}

impl fmt::Display for PciAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PCI:{:#018x}", self.0)
    }
}

impl fmt::Display for PciCpuAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CPU:{:#018x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_pci_addr_basic() {
        let addr = PciAddr::new(0x4000_0000);
        assert_eq!(addr.as_usize(), 0x4000_0000);
    }

    #[test_case]
    fn test_pci_cpu_addr_basic() {
        let addr = PciCpuAddr::new(0x3000_0000);
        assert_eq!(addr.as_usize(), 0x3000_0000);
    }

    #[test_case]
    fn test_pci_to_cpu_conversion() {
        let pci_addr = PciAddr::new(0x4000_0000);
        let cpu_addr = pci_addr.to_cpu_addr(-0x1000_0000);
        assert_eq!(cpu_addr.as_usize(), 0x3000_0000);
    }

    #[test_case]
    fn test_cpu_to_phys_conversion() {
        let cpu_addr = PciCpuAddr::new(0x3000_0000);
        let phys_addr = cpu_addr.as_phys_addr();
        assert_eq!(phys_addr.as_usize(), 0x3000_0000);
    }

    #[test_case]
    fn test_display_format() {
        let pci = PciAddr::new(0x4000_0000);
        let cpu = PciCpuAddr::new(0x3000_0000);

        let pci_str = alloc::format!("{}", pci);
        let cpu_str = alloc::format!("{}", cpu);

        assert!(pci_str.starts_with("PCI:"));
        assert!(cpu_str.starts_with("CPU:"));
    }
}
