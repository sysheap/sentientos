use crate::{
    drivers::virtio::capability::{VirtioPciCap, VIRTIO_PCI_CAP_COMMON_CFG},
    info,
    klibc::MMIO,
    pci::{command_register, GeneralDevicePciHeader, PCIBitField, PCIInformation},
};
use alloc::vec::Vec;

const VIRTIO_VENDOR_SPECIFIC_CAPABILITY_ID: u8 = 0x9;

const DEVICE_STATUS_ACKNOWLEDGE: u8 = 1;
const DEVICE_STATUS_DRIVER: u8 = 2;
const DEVICE_STATUS_DRIVER_OK: u8 = 4;
const DEVICE_STATUS_FEATURES_OK: u8 = 8;
const DEVICE_STATUS_FAILED: u8 = 128;
const DEVICE_STATUS_DEVICE_NEEDS_RESTARTL: u8 = 64;

const VIRTIO_NET_F_CSUM: u64 = 1 << 0;
const VIRTIO_NET_F_MAC: u64 = 1 << 5;
const VIRTIO_F_VERSION_1: u64 = 1 << 32;

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

        let mut common_cfg: MMIO<VirtioPciCommonCfg> =
            unsafe { MMIO::new(config_bar.cpu_address + common_cfg.offset()) };

        info!("Common config: {:#x?}", *common_cfg);

        // Let's try to initialize the device
        common_cfg.device_status = 0x0;
        while common_cfg.device_status != 0x0 {}

        common_cfg.device_status = common_cfg.device_status | DEVICE_STATUS_ACKNOWLEDGE;
        common_cfg.device_status = common_cfg.device_status | DEVICE_STATUS_DRIVER;

        // Read features and write subset to it
        common_cfg.device_feature_select = 0;
        let mut device_features = common_cfg.device_feature as u64;

        common_cfg.device_feature_select = 1;
        device_features |= (common_cfg.device_feature as u64) << 32;

        assert!(
            device_features & VIRTIO_F_VERSION_1 != 0,
            "Virtio version 1 not supported"
        );

        let wanted_features: u64 = VIRTIO_F_VERSION_1 | VIRTIO_NET_F_MAC;

        assert!(
            device_features & wanted_features == wanted_features,
            "Device does not support wanted features"
        );

        common_cfg.driver_feature_select = 0;
        common_cfg.driver_feature = wanted_features as u32;

        common_cfg.driver_feature_select = 1;
        common_cfg.driver_feature = (wanted_features >> 32) as u32;

        common_cfg.device_status = common_cfg.device_status | DEVICE_STATUS_FEATURES_OK;

        assert!(
            common_cfg.device_status & DEVICE_STATUS_FEATURES_OK != 0,
            "Device features not ok"
        );

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
    config_msix_vector: u16,
    num_queues: u16,
    device_status: u8,
    config_generation: u8,
    /* About a specific virtqueue. */
    queue_select: u16,
    queue_size: u16,
    queue_msix_vector: u16,
    queue_enable: u16,
    queue_notify_off: u16,
    queue_desc: u64,
    queue_driver: u64,
    queue_device: u64,
    queue_notify_data: u16,
    queue_reset: u16,
}
