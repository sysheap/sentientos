use crate::{info, klibc::MMIO};

mod devic_tree_parser;
mod lookup;

use lookup::lookup;

pub use devic_tree_parser::parse;

use self::devic_tree_parser::PCIInformation;

pub fn enumerate_devices(pci_information: &PCIInformation) {
    for bus in 0..255 {
        for device in 0..32 {
            for function in 0..8 {
                let address = pci_address(
                    pci_information.pci_host_bridge_address,
                    bus,
                    device,
                    function,
                );
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
