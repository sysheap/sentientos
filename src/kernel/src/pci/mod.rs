use crate::{device_tree::StructureBlockIterator, info, klibc::MMIO};

mod lookup;

use lookup::lookup;

pub fn enumerate_devices(pci_host_bridge_address: usize) {
    for bus in 0..255 {
        for device in 0..32 {
            for function in 0..8 {
                let address = pci_address(pci_host_bridge_address, bus, device, function);
                let mmio: MMIO<u32> = MMIO::new(address);
                let header = unsafe { mmio.read() };
                if header != 0xffff_ffff {
                    let vendor_id = header as u16;
                    let device_id = (header >> 16) as u16;
                    let name = lookup(vendor_id, device_id).expect("PCI Device must be known.");
                    info!(
                        "PCI Device {:#x}:{:#x} found at {:#x} ({})",
                        vendor_id, device_id, address, name
                    );
                }
            }
        }
    }
}

fn pci_address(starting_address: usize, bus: u8, device: u8, function: u8) -> usize {
    assert!(device < 32);
    assert!(function < 8);
    let offset = (bus as usize) << 20 | (device as usize) << 15 | (function as usize) << 12;
    starting_address + offset
}

pub fn get_pci_host_bridge_address(dt_structure_block: StructureBlockIterator) -> Option<usize> {
    for node in dt_structure_block {
        if let crate::device_tree::FdtToken::BeginNode(node_name) = node
            && node_name.starts_with("pci")
        {
            let address = node_name
                .split('@')
                .nth(1)
                .and_then(|addr| usize::from_str_radix(addr, 16).ok())
                .expect("The pci node should have an address");
            info!("PCI Host Address: {:#x}", address);
            return Some(address);
        }
    }
    None
}
