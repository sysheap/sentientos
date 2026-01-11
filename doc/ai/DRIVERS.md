# Device Drivers

## Overview

Device driver subsystems:
1. **PCI** - PCI device enumeration and configuration
2. **VirtIO** - VirtIO device framework (network)

## PCI Subsystem

**File:** `kernel/src/pci/mod.rs`

### PCI Device Discovery

```rust
pub fn enumerate_devices(pci_info: &PCIInformation) -> EnumeratedDevices {
    // Scan PCI configuration space
    // Find devices by vendor/device ID
    // Return categorized devices (network, storage, etc.)
}
```

### PCI Header Structure

```rust
struct GeneralDevicePciHeader {
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
    expansion_rom_base_address: u32,
    capabilities_pointer: u8,
}
```

### PCIDevice

```rust
pub struct PCIDevice {
    header: MMIO<GeneralDevicePciHeader>,
    bars: BTreeMap<u8, PCIAllocatedSpace>,
}

impl PCIDevice {
    pub fn capabilities(&self) -> PciCapabilityIter
    pub fn get_or_initialize_bar(&mut self, index: u8) -> &PCIAllocatedSpace
}
```

### PCI Constants

```rust
const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_DEVICE_ID: RangeInclusive<u16> = 0x1000..=0x107F;
const VIRTIO_NETWORK_SUBSYSTEM_ID: u16 = 1;
```

### PCI Allocator

**File:** `kernel/src/pci/allocator.rs`

Allocates 64-bit memory space for BAR configuration:

```rust
pub static PCI_ALLOCATOR_64_BIT: Spinlock<PCIAllocator> = Spinlock::new(PCIAllocator::new());

impl PCIAllocator {
    pub fn init(&mut self, range: &PCIRange)
    pub fn allocate(&mut self, size: usize) -> Option<PCIAllocatedSpace>
}
```

### Device Tree Parser

**File:** `kernel/src/pci/devic_tree_parser.rs`

Parses PCI information from device tree:

```rust
pub fn parse() -> Result<PCIInformation, PCIError>

pub struct PCIInformation {
    pub pci_host_bridge_address: usize,
    pub pci_host_bridge_length: usize,
    pub ranges: Vec<PCIRange>,
}
```

## VirtIO Framework

**File:** `kernel/src/drivers/virtio/`

### VirtIO Network Device

**File:** `kernel/src/drivers/virtio/net/mod.rs`

```rust
pub struct NetworkDevice {
    device: PCIDevice,
    common_cfg: MMIO<virtio_pci_common_cfg>,
    net_cfg: MMIO<virtio_net_config>,
    notify_cfg: MMIO<virtio_pci_notify_cap>,
    transmit_queue: VirtQueue<EXPECTED_QUEUE_SIZE>,
    receive_queue: VirtQueue<EXPECTED_QUEUE_SIZE>,
    mac_address: MacAddress,
}

impl NetworkDevice {
    pub fn initialize(pci_device: PCIDevice) -> Result<Self, &'static str>
    pub fn send_packet(&mut self, packet: Vec<u8>) -> Result<(), QueueError>
    pub fn receive_packets(&mut self) -> Vec<Vec<u8>>
    pub fn get_mac_address(&self) -> MacAddress
}
```

### VirtIO Initialization

```rust
pub fn initialize(mut pci_device: PCIDevice) -> Result<Self, &'static str> {
    // 1. Find VirtIO capabilities
    let common_cfg = find_capability(VIRTIO_PCI_CAP_COMMON_CFG)?;
    let net_cfg = find_capability(VIRTIO_PCI_CAP_DEVICE_CFG)?;
    let notify_cfg = find_capability(VIRTIO_PCI_CAP_NOTIFY_CFG)?;

    // 2. Reset device
    common_cfg.device_status().write(0x0);

    // 3. Set ACKNOWLEDGE status
    device_status |= DEVICE_STATUS_ACKNOWLEDGE;

    // 4. Set DRIVER status
    device_status |= DEVICE_STATUS_DRIVER;

    // 5. Negotiate features
    let features = VIRTIO_NET_F_MAC | VIRTIO_F_VERSION_1;
    common_cfg.driver_feature().write(features);

    // 6. Set FEATURES_OK
    device_status |= DEVICE_STATUS_FEATURES_OK;

    // 7. Set up virtqueues
    let receive_queue = VirtQueue::new(...);
    let transmit_queue = VirtQueue::new(...);

    // 8. Set DRIVER_OK
    device_status |= DEVICE_STATUS_DRIVER_OK;
}
```

### VirtQueue

**File:** `kernel/src/drivers/virtio/virtqueue.rs`

Ring buffer for device communication:

```rust
pub struct VirtQueue<const SIZE: usize> {
    descriptors: &'static mut [VirtQueueDescriptor; SIZE],
    available_ring: &'static mut VirtQueueAvailableRing<SIZE>,
    used_ring: &'static mut VirtQueueUsedRing<SIZE>,
    // ...
}

impl<const SIZE: usize> VirtQueue<SIZE> {
    pub fn new(queue_index: u16, notify_offset: usize) -> Self
    pub fn add_buffer(&mut self, buffer: &[u8], direction: BufferDirection)
    pub fn get_used_buffers(&mut self) -> impl Iterator<Item = Vec<u8>>
}
```

### VirtIO Capabilities

**File:** `kernel/src/drivers/virtio/capability.rs`

```rust
pub const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 1;
pub const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 2;
pub const VIRTIO_PCI_CAP_ISR_CFG: u8 = 3;
pub const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 4;
pub const VIRTIO_PCI_CAP_PCI_CFG: u8 = 5;
```

## MMIO Utilities

**File:** `kernel/src/klibc/mmio.rs`

Memory-mapped I/O helper:

```rust
pub struct MMIO<T>(*mut T);

impl<T> MMIO<T> {
    pub fn new(addr: usize) -> Self
    pub fn read(&self) -> T
    pub fn write(&self, value: T)
}
```

### mmio_struct! Macro

Generates MMIO accessors for struct fields:

```rust
mmio_struct! {
    struct GeneralDevicePciHeader {
        vendor_id: u16,
        device_id: u16,
        // ...
    }
}

// Generates:
impl MMIO<GeneralDevicePciHeader> {
    pub fn vendor_id(&self) -> MMIO<u16>
    pub fn device_id(&self) -> MMIO<u16>
}
```

## Key Files

| File | Purpose |
|------|---------|
| kernel/src/pci/mod.rs | PCI enumeration |
| kernel/src/pci/allocator.rs | BAR space allocation |
| kernel/src/pci/devic_tree_parser.rs | Device tree PCI info |
| kernel/src/pci/lookup.rs | Device ID lookup |
| kernel/src/drivers/virtio/mod.rs | VirtIO module |
| kernel/src/drivers/virtio/net/mod.rs | VirtIO network driver |
| kernel/src/drivers/virtio/virtqueue.rs | VirtQueue implementation |
| kernel/src/drivers/virtio/capability.rs | VirtIO capability parsing |
| kernel/src/klibc/mmio.rs | MMIO utilities |

## Adding a New VirtIO Driver

1. Find device during PCI enumeration by subsystem ID
2. Parse VirtIO capabilities from PCI config space
3. Initialize common config (reset, acknowledge, negotiate features)
4. Set up virtqueues for communication
5. Set DRIVER_OK status
6. Implement send/receive via virtqueues
