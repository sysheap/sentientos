use crate::{
    drivers::virtio::capability::VirtioPciCap, info, klibc::MMIO, pci::GeneralDevicePciHeader,
};

const VIRTIO_VENDOR_SPECIFIC_CAPABILITY_ID: u8 = 0x9;

pub struct NetworkDevice {
    device: MMIO<GeneralDevicePciHeader>,
}

impl NetworkDevice {
    pub fn initialize(pci_device: MMIO<GeneralDevicePciHeader>) -> Result<Self, &'static str> {
        let capabilities = pci_device.capabilities();
        let virtio_capabilities = capabilities
            .filter(|cap| cap.id() == VIRTIO_VENDOR_SPECIFIC_CAPABILITY_ID)
            .map(|cap| unsafe { cap.new_type::<VirtioPciCap>() });
        info!("Network device has following VirtIO capabilities");
        for capability in virtio_capabilities {
            info!("Found capability {:?}", *capability);
        }
        Ok(Self { device: pci_device })
    }
}
