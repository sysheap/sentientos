// cfg_type values
/* Common configuration */
pub const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 1;
/* Notifications */
pub const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 2;
/* ISR Status */
pub const VIRTIO_PCI_CAP_ISR_CFG: u8 = 3;
/* Device specific configuration */
pub const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 4;
/* PCI configuration access */
pub const VIRTIO_PCI_CAP_PCI_CFG: u8 = 5;
/* Shared memory region */
pub const VIRTIO_PCI_CAP_SHARED_MEMORY_CFG: u8 = 8;
/* Vendor-specific data */
pub const VIRTIO_PCI_CAP_VENDOR_CFG: u8 = 9;

#[derive(Debug)]
#[repr(packed)]
pub(super) struct VirtioPciCap {
    cap_vndr: u8,     /* Generic PCI field: PCI_CAP_ID_VNDR */
    cap_next: u8,     /* Generic PCI field: next ptr. */
    cap_len: u8,      /* Generic PCI field: capability length */
    cfg_type: u8,     /* Identifies the structure. */
    bar: u8,          /* Where to find it. */
    id: u8,           /* Multiple capabilities of the same type */
    padding: [u8; 2], /* Pad to full dword. */
    offset: u32,      /* Offset within bar. */
    length: u32,      /* Length of the structure, in bytes. */
}

impl VirtioPciCap {
    pub fn cfg_type(&self) -> u8 {
        self.cfg_type
    }

    pub fn bar(&self) -> u8 {
        self.bar
    }
}
