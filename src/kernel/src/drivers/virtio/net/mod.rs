use crate::{info, klibc::MMIO, pci::GeneralDevicePciHeader};

pub struct NetworkDevice {
    device: MMIO<GeneralDevicePciHeader>,
}

impl NetworkDevice {
    pub fn initialize(pci_device: MMIO<GeneralDevicePciHeader>) -> Result<Self, &'static str> {
        let capabilities = pci_device.capabilities();
        info!("Network device has following capabilities");
        for capability in capabilities {
            info!("Found capability {:?}", *capability);
        }
        Ok(Self { device: pci_device })
    }
}
