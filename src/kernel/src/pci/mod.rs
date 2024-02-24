use crate::{device_tree::StructureBlockIterator, info, klibc::MMIO};

pub fn enumerate_devices(dt_structure_block: StructureBlockIterator) {
    let root_addre =
        get_pci_address(dt_structure_block).expect("Could not retrieve pci host address");
    info!("PCI Host Address: {:#x}", root_addre);
    for bus in 0..255 {
        for device in 0..32 {
            for function in 0..8 {
                let address = pci_address(root_addre, bus, device, function);
                let mmio: MMIO<u32> = MMIO::new(address);
                let header = unsafe { mmio.read() };
                if header != 0xffff_ffff {
                    info!(
                        "PCI Device {:#x}:{:#x} found at {:#x}",
                        header & 0xffff,
                        (header >> 16) & 0xffff,
                        address
                    );
                }
            }
        }
    }
}

fn pci_address(starting_address: usize, bus: u8, device: u8, function: u8) -> usize {
    assert!(device < 32);
    assert!(function < 8);
    starting_address + ((bus as usize) << 20 | (device as usize) << 15 | (function as usize) << 12)
}

fn get_pci_address(dt_structure_block: StructureBlockIterator) -> Option<usize> {
    for node in dt_structure_block {
        if let crate::device_tree::FdtToken::BeginNode(node_name) = node
            && node_name.starts_with("pci")
        {
            let address = node_name
                .split('@')
                .nth(1)
                .map(|addr| usize::from_str_radix(addr, 16).ok())
                .flatten()
                .expect("The pci node should have an address");
            return Some(address);
        }
    }
    None
}
