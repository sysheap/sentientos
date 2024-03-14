use crate::{
    drivers::virtio::capability::{VirtioPciCap, VIRTIO_PCI_CAP_COMMON_CFG},
    info,
    klibc::MMIO,
    pci::{command_register, GeneralDevicePciHeader, PCIBitField, PCIInformation, PCIRange},
};
use alloc::vec::Vec;

const VIRTIO_VENDOR_SPECIFIC_CAPABILITY_ID: u8 = 0x9;

pub struct NetworkDevice {
    device: MMIO<GeneralDevicePciHeader>,
}

impl NetworkDevice {
    pub fn initialize(
        pci_information: &PCIInformation,
        mut pci_device: MMIO<GeneralDevicePciHeader>,
    ) -> Result<Self, &'static str> {
        let capabilities = pci_device.capabilities();
        let virtio_capabilities: Vec<MMIO<VirtioPciCap>> = capabilities
            .filter(|cap| cap.id() == VIRTIO_VENDOR_SPECIFIC_CAPABILITY_ID)
            .map(|cap| unsafe { cap.new_type::<VirtioPciCap>() })
            .collect();
        info!("Network device has following VirtIO capabilities");
        for capability in &virtio_capabilities {
            info!("Found capability {:?}", **capability);
        }

        let common_cfg = virtio_capabilities
            .iter()
            .find(|cap| cap.cfg_type() == VIRTIO_PCI_CAP_COMMON_CFG)
            .ok_or("Common configuration capability not found")?;

        info!(
            "Common configuration capability found at {:?}",
            **common_cfg
        );

        // Disable I/O space and memory space to determine bar size
        pci_device.clear_command_register_bits(
            command_register::IO_SPACE | command_register::MEMORY_SPACE,
        );

        let bar_index = common_cfg.bar();

        let original_bar_value = pci_device.bar(bar_index);

        assert!(original_bar_value & 0x1 == 0, "Bar must be memory mapped");
        assert!(
            (original_bar_value & 0b110) >> 1 == 0x2,
            "Bar must be 64-bit wide"
        );

        pci_device.write_bar(bar_index, 0xffffffff);
        let bar_value = pci_device.bar(bar_index);

        // Mask out the 4 lower bits because they describe the type of the bar
        // Invert the value and add 1 to get the size (because the bits that are not set are zero because of alignment)
        let size = !(bar_value & !0b1111) + 1;

        info!(
            "Bar {} value: {:#x} size: {:#x}",
            bar_index, bar_value, size
        );

        // Let's use some pci address space
        let range = pci_information
            .get_first_range_for_type(PCIBitField::MEMORY_SPACE_64_BIT_CODE)
            .unwrap();

        info!("Range: {:#x?}", range);

        assert!(
            range.pci_child_address % (size as usize) == 0,
            "Address must be aligned"
        );

        pci_device.write_bar(bar_index, range.pci_child_address as u32);
        pci_device.write_bar(bar_index + 1, (range.pci_child_address >> 32) as u32);

        pci_device.set_command_register_bits(command_register::MEMORY_SPACE);

        let common_cfg: MMIO<VirtioPciCommonCfg> = unsafe { MMIO::new(range.parent_address) };

        info!("Common config: {:#x?}", *common_cfg);

        Ok(Self { device: pci_device })
    }
}

#[derive(Debug)]
#[repr(C)]
struct VirtioPciCommonCfg {
    device_feature_select: u32,
    device_feature: u32,
    driver_feature_select: u32,
    driver_feature: u32,
    config_msix_vector: u32,
    num_queues: u32,
    device_status: u8,
    config_generation: u8,
}
