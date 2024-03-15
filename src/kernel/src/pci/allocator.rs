use core::ops::Range;

use common::util;

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
        self.free_space_pci_space =
            pci_range.pci_address..pci_range.pci_address + pci_range.size as usize;
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
            cpu_address: (aligned_current as i64 + self.offset_to_cpu_space) as usize,
            size,
        })
    }
}

pub struct PCIAllocatedSpace {
    pub pci_address: usize,
    pub cpu_address: usize,
    pub size: usize,
}
