use crate::{info, klibc::MMIO};
use alloc::vec::Vec;

mod devic_tree_parser;
mod lookup;

use lookup::lookup;

pub use devic_tree_parser::parse;

use self::devic_tree_parser::PCIInformation;

const SUBSYSTEM_ID_OFFSET: usize = 0x2e;
const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_DEVICE_ID: core::ops::Range<u16> = 0x1000..0x103F;
const VIRTIO_NETWORK_SUBSYSTEM_ID: u16 = 1;

pub type PciAddress = usize;

#[derive(Debug)]
pub struct PciDeviceAddresses {
    network_devices: Vec<PciAddress>,
}

impl PciDeviceAddresses {
    fn new() -> Self {
        Self {
            network_devices: Vec::new(),
        }
    }
}

pub fn enumerate_devices(pci_information: &PCIInformation) -> PciDeviceAddresses {
    let mut pci_devices = PciDeviceAddresses::new();
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

                    let subsystem_id_address: MMIO<u16> = MMIO::new(address + SUBSYSTEM_ID_OFFSET);
                    let subsystem_id = unsafe { subsystem_id_address.read() };

                    // Add virtio devices to device list
                    if vendor_id == VIRTIO_VENDOR_ID
                        && VIRTIO_DEVICE_ID.contains(&device_id)
                        && subsystem_id == VIRTIO_NETWORK_SUBSYSTEM_ID
                    {
                        pci_devices.network_devices.push(address);
                    }
                }
            }
        }
    }
    pci_devices
}

fn pci_address(starting_address: usize, bus: u8, device: u8, function: u8) -> usize {
    assert!(device < 32);
    assert!(function < 8);
    let offset = (bus as usize) << 20 | (device as usize) << 15 | (function as usize) << 12;
    starting_address + offset
}
