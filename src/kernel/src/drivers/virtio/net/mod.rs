use crate::{
    drivers::virtio::capability::{VirtioPciCap, VIRTIO_PCI_CAP_COMMON_CFG},
    info,
    klibc::MMIO,
    pci::{command_register, GeneralDevicePciHeader, PCIBitField, PCIInformation},
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

        let bar_index = common_cfg.bar();

        let config_bar = pci_device.initialize_bar(bar_index);

        let common_cfg: MMIO<VirtioPciCommonCfg> =
            unsafe { MMIO::new(config_bar.cpu_address + common_cfg.offset()) };

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
