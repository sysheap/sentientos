use crate::{info, klibc::MMIO};
use alloc::vec::Vec;

mod devic_tree_parser;
mod lookup;

use lookup::lookup;

pub use devic_tree_parser::parse;

use self::devic_tree_parser::PCIInformation;

const INVALID_VENDOR_ID: u16 = 0xffff;

const GENERAL_DEVICE_TYPE: u8 = 0x0;
const GENERAL_DEVICE_TYPE_MASK: u8 = !0x80;

const CAPABILITY_POINTER_MASK: u8 = !0x3;

const SUBSYSTEM_ID_OFFSET: usize = 0x2e;
const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_DEVICE_ID: core::ops::RangeInclusive<u16> = 0x1000..=0x107F;
const VIRTIO_NETWORK_SUBSYSTEM_ID: u16 = 1;

#[repr(packed)]
#[allow(dead_code)]
pub struct GeneralDevicePciHeader {
    vendor_id: u16,
    device_id: u16,
    command_register: u16,
    status_register: u16,
    revision_id: u8,
    programming_interface_byte: u8,
    subclass: u8,
    class_code: u8,
    cache_line_size: u8,
    latency_timer: u8,
    header_type: u8,
    built_in_self_test: u8,
    bar0: u32,
    bar1: u32,
    bar2: u32,
    bar3: u32,
    bar4: u32,
    bar5: u32,
    cardbus_cis_pointer: u32,
    subsystem_vendor_id: u16,
    subsystem_id: u16,
    expnasion_rom_base_address: u32,
    capabilities_pointer: u8,
}

pub struct PciCapabilityIter<'a> {
    pci_device: &'a MMIO<GeneralDevicePciHeader>,
    next_offset: u8, // 0 means there is no next pointer
}

#[derive(Debug)]
#[repr(packed)]
pub struct PciCapability {
    id: u8,
    next: u8,
}

impl PciCapability {
    pub fn id(&self) -> u8 {
        self.id
    }
}

impl<'a> Iterator for PciCapabilityIter<'a> {
    type Item = MMIO<PciCapability>;

    fn next(&mut self) -> Option<Self::Item> {
        let capability: MMIO<PciCapability> = match self.next_offset {
            0 => return None,
            _ => unsafe {
                self.pci_device
                    .new_type_with_offset(self.next_offset as usize)
            },
        };
        self.next_offset = capability.next;
        Some(capability)
    }
}

impl MMIO<GeneralDevicePciHeader> {
    unsafe fn try_new(address: usize) -> Option<Self> {
        let pci_device = Self::new(address);
        if pci_device.vendor_id == INVALID_VENDOR_ID {
            return None;
        }
        assert!(pci_device.header_type & GENERAL_DEVICE_TYPE_MASK == GENERAL_DEVICE_TYPE);
        Some(pci_device)
    }

    const CAPABILITIES_LIST_BIT: u16 = 1 << 4;
    pub fn capabilities(&self) -> PciCapabilityIter {
        if self.status_register & Self::CAPABILITIES_LIST_BIT == 0 {
            PciCapabilityIter {
                pci_device: self,
                next_offset: 0,
            }
        } else {
            PciCapabilityIter {
                pci_device: self,
                next_offset: self.capabilities_pointer & CAPABILITY_POINTER_MASK,
            }
        }
    }
}

#[derive(Debug)]
pub struct PciDeviceAddresses {
    pub network_devices: Vec<MMIO<GeneralDevicePciHeader>>,
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
                let maybe_device = unsafe { MMIO::try_new(address) };
                if let Some(device) = maybe_device {
                    let vendor_id = device.vendor_id;
                    let device_id = device.device_id;
                    let name = lookup(vendor_id, device_id).expect("PCI Device must be known.");
                    info!(
                        "PCI Device {:#x}:{:#x} found at {:#x} ({})",
                        vendor_id, device_id, address, name
                    );

                    let subsystem_id_address: MMIO<u16> =
                        unsafe { MMIO::new(address + SUBSYSTEM_ID_OFFSET) };
                    let subsystem_id = *subsystem_id_address;

                    // Add virtio devices to device list
                    if vendor_id == VIRTIO_VENDOR_ID
                        && VIRTIO_DEVICE_ID.contains(&device_id)
                        && subsystem_id == VIRTIO_NETWORK_SUBSYSTEM_ID
                    {
                        pci_devices
                            .network_devices
                            .push(unsafe { MMIO::new(address) });
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
