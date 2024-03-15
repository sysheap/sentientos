use crate::assert::static_assert_size;
use crate::{debug, pci};
use crate::{info, klibc::MMIO};
use alloc::vec::Vec;

mod allocator;
mod devic_tree_parser;
mod lookup;

use common::mutex::Mutex;
use lookup::lookup;

pub use devic_tree_parser::parse;

use self::allocator::{PCIAllocatedSpace, PCIAllocator};
pub use self::devic_tree_parser::PCIBitField;
pub use self::devic_tree_parser::PCIInformation;
pub use self::devic_tree_parser::PCIRange;

pub static PCI_ALLOCATOR_64_BIT: Mutex<PCIAllocator> = Mutex::new(PCIAllocator::new());

const INVALID_VENDOR_ID: u16 = 0xffff;

const GENERAL_DEVICE_TYPE: u8 = 0x0;
const GENERAL_DEVICE_TYPE_MASK: u8 = !0x80;

const CAPABILITY_POINTER_MASK: u8 = !0x3;

const SUBSYSTEM_ID_OFFSET: usize = 0x2e;
const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_DEVICE_ID: core::ops::RangeInclusive<u16> = 0x1000..=0x107F;
const VIRTIO_NETWORK_SUBSYSTEM_ID: u16 = 1;

pub mod command_register {
    pub const IO_SPACE: u16 = 1 << 0;
    pub const MEMORY_SPACE: u16 = 1 << 1;
}

#[repr(C)]
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
    bars: [u32; 6],
    cardbus_cis_pointer: u32,
    subsystem_vendor_id: u16,
    subsystem_id: u16,
    expnasion_rom_base_address: u32,
    capabilities_pointer: u8,
}

impl GeneralDevicePciHeader {
    pub fn bar(&self, index: u8) -> u32 {
        assert!(index < 6);
        self.bars[index as usize]
    }

    pub fn write_bar(&mut self, index: u8, value: u32) {
        assert!(index < 6);
        self.bars[index as usize] = value;
    }

    pub fn initialize_bar(&mut self, index: u8) -> PCIAllocatedSpace {
        self.clear_command_register_bits(
            command_register::IO_SPACE | command_register::MEMORY_SPACE,
        );

        let original_bar_value = self.bar(index);
        assert!(original_bar_value & 0x1 == 0, "Bar must be memory mapped");
        assert!(
            (original_bar_value & 0b110) >> 1 == 0x2,
            "Bar must be 64-bit wide"
        );

        // Determine size of bar
        self.write_bar(index, 0xffffffff);
        let bar_value = self.bar(index);

        // Mask out the 4 lower bits because they describe the type of the bar
        // Invert the value and add 1 to get the size (because the bits that are not set are zero because of alignment)
        let size = !(bar_value & !0b1111) + 1;

        debug!("Bar {} size: {:#x}", index, size);

        let space = pci::PCI_ALLOCATOR_64_BIT
            .lock()
            .allocate(size as usize)
            .expect("There must be enough space for the bar");

        self.write_bar(index, space.pci_address as u32);
        self.write_bar(index + 1, (space.pci_address >> 32) as u32);

        self.set_command_register_bits(command_register::MEMORY_SPACE);

        space
    }

    pub fn set_command_register_bits(&mut self, bits: u16) {
        self.command_register |= bits;
    }

    pub fn clear_command_register_bits(&mut self, bits: u16) {
        self.command_register &= !bits;
    }
}

pub struct PciCapabilityIter<'a> {
    pci_device: &'a MMIO<GeneralDevicePciHeader>,
    next_offset: u8, // 0 means there is no next pointer
}

#[derive(Debug)]
#[repr(C)]
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
