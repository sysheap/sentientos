use core::ops::Range;

use crate::klibc::util;

use super::PCIRange;

/// This struct will store the free space in the PCI address space and the offset to the CPU address space.
/// It will be used as a very simple one-way allocator. We assume we never ran out of space and therefore ignore
/// the freeing logic.
pub struct PCIAllocator {
    free_space_pci_space: Range<usize>,
    offset_to_cpu_space: i64,
}

impl PCIAllocator {
    pub const fn new() -> Self {
        Self {
            free_space_pci_space: 0..0,
            offset_to_cpu_space: 0,
        }
    }

    pub fn init(&mut self, pci_range: &PCIRange) {
        self.free_space_pci_space = pci_range.pci_address..pci_range.pci_address + pci_range.size;
        self.offset_to_cpu_space = pci_range.cpu_address as i64 - pci_range.pci_address as i64;
    }

    pub fn allocate(&mut self, size: usize) -> Option<PCIAllocatedSpace> {
        let current = self.free_space_pci_space.start;
        let aligned_current = util::align_up(current, size);
        if aligned_current + size > self.free_space_pci_space.end {
            return None;
        }
        self.free_space_pci_space.start = aligned_current + size;
        Some(PCIAllocatedSpace {
            pci_address: aligned_current,
            cpu_address: util::wrapping_add_signed(aligned_current, self.offset_to_cpu_space),
            size,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct PCIAllocatedSpace {
    pub pci_address: usize,
    pub cpu_address: usize,
    pub size: usize,
}

#[cfg(test)]
mod tests {
    use crate::pci::{PCIBitField, PCIRange};

    use super::PCIAllocator;

    fn init_allocator(size: usize) -> PCIAllocator {
        let mut allocator = PCIAllocator::new();
        allocator.init(&PCIRange {
            cpu_address: 4096,
            pci_address: 8192,
            size,
            pci_bitfield: PCIBitField::from(0),
        });
        allocator
    }

    #[test_case]
    fn empty_allocator() {
        let mut allocator = PCIAllocator::new();
        assert!(
            allocator.allocate(0x100).is_none(),
            "Empty allocator must be none"
        );
    }

    #[test_case]
    fn alignment() {
        let mut allocator = init_allocator(8192);
        let _ = allocator
            .allocate(3)
            .expect("Small allocation must succeed");
        let allocation = allocator
            .allocate(4096)
            .expect("Page-sized allocation must succeed");
        assert!(
            allocation.cpu_address.is_multiple_of(4096),
            "cpu address must be properly aligned"
        );
        assert!(
            allocation.pci_address.is_multiple_of(4096),
            "pci address must be properly aligned"
        );
    }

    #[test_case]
    fn exhausted() {
        let mut allocator = init_allocator(128);
        assert!(allocator.allocate(64).is_some());
        assert!(allocator.allocate(128).is_none());
        assert!(allocator.allocate(64).is_some());
        assert!(allocator.allocate(1).is_none());
    }
}
