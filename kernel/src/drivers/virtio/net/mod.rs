use crate::{
    assert::static_assert_size,
    debug,
    drivers::virtio::{
        capability::{
            VIRTIO_PCI_CAP_COMMON_CFG, VIRTIO_PCI_CAP_DEVICE_CFG, VIRTIO_PCI_CAP_NOTIFY_CFG,
            virtio_pci_cap,
        },
        virtqueue::{BufferDirection, VirtQueue},
    },
    info,
    klibc::{
        MMIO,
        util::{BufferExtension, ByteInterpretable, is_power_of_2_or_zero},
    },
    mmio_struct,
    net::mac::MacAddress,
    pci::PCIDevice,
};
use alloc::vec::Vec;

use super::virtqueue::QueueError;

const EXPECTED_QUEUE_SIZE: usize = 0x100;

const VIRTIO_VENDOR_SPECIFIC_CAPABILITY_ID: u8 = 0x9;

const DEVICE_STATUS_ACKNOWLEDGE: u8 = 1;
const DEVICE_STATUS_DRIVER: u8 = 2;
const DEVICE_STATUS_DRIVER_OK: u8 = 4;
const DEVICE_STATUS_FEATURES_OK: u8 = 8;
const DEVICE_STATUS_FAILED: u8 = 128;
#[allow(dead_code)]
const DEVICE_STATUS_DEVICE_NEEDS_RESTART: u8 = 64;

const VIRTIO_NET_F_MAC: u64 = 1 << 5;
const VIRTIO_F_VERSION_1: u64 = 1 << 32;

#[allow(dead_code)]
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
    pub fn initialize(mut pci_device: PCIDevice) -> Result<Self, &'static str> {
        let capabilities = pci_device.capabilities();
        let mut virtio_capabilities: Vec<MMIO<virtio_pci_cap>> = capabilities
            .filter(|cap| cap.id().read() == VIRTIO_VENDOR_SPECIFIC_CAPABILITY_ID)
            .map(|cap| unsafe { cap.new_type::<virtio_pci_cap>() })
            .collect();

        let common_cfg = virtio_capabilities
            .iter()
            .find(|cap| cap.cfg_type().read() == VIRTIO_PCI_CAP_COMMON_CFG)
            .ok_or("Common configuration capability not found")?;

        debug!("Common configuration capability found at {:?}", common_cfg);

        let config_bar = pci_device.get_or_initialize_bar(common_cfg.bar().read());

        let common_cfg: MMIO<virtio_pci_common_cfg> =
            MMIO::new(config_bar.cpu_address + common_cfg.offset().read() as usize);

        debug!("Common config: {:#x?}", common_cfg);

        // Let's try to initialize the device
        common_cfg.device_status().write(0x0);

        #[allow(clippy::while_immutable_condition)]
        while common_cfg.device_status().read() != 0x0 {}

        let mut device_status = common_cfg.device_status();
        device_status |= DEVICE_STATUS_ACKNOWLEDGE;

        assert!(
            common_cfg.device_status().read() & DEVICE_STATUS_FAILED == 0,
            "Device failed"
        );

        device_status |= DEVICE_STATUS_DRIVER;

        assert!(
            common_cfg.device_status().read() & DEVICE_STATUS_FAILED == 0,
            "Device failed"
        );

        // Read features and write subset to it
        common_cfg.device_feature_select().write(0);
        let mut device_features = common_cfg.device_feature().read() as u64;

        common_cfg.device_feature_select().write(1);
        device_features |= (common_cfg.device_feature().read() as u64) << 32;

        assert!(
            device_features & VIRTIO_F_VERSION_1 != 0,
            "Virtio version 1 not supported"
        );

        let wanted_features: u64 = VIRTIO_F_VERSION_1 | VIRTIO_NET_F_MAC;

        assert!(
            device_features & wanted_features == wanted_features,
            "Device does not support wanted features"
        );

        common_cfg.driver_feature_select().write(0);
        common_cfg.driver_feature().write(wanted_features as u32);

        common_cfg.driver_feature_select().write(1);
        common_cfg
            .driver_feature()
            .write((wanted_features >> 32) as u32);

        device_status |= DEVICE_STATUS_FEATURES_OK;

        assert!(
            device_status.read() & DEVICE_STATUS_FAILED == 0,
            "Device failed"
        );

        assert!(
            device_status.read() & DEVICE_STATUS_FEATURES_OK != 0,
            "Device features not ok"
        );

        // Get notification configuration
        let notify_cfg = virtio_capabilities
            .iter()
            .find(|cap| cap.cfg_type().read() == VIRTIO_PCI_CAP_NOTIFY_CFG)
            .ok_or("Notification capability not found")?;

        // SAFTEY: Notification capability is a different type
        let notify_cfg = unsafe { notify_cfg.new_type::<virtio_pci_notify_cap>() };

        assert!(
            is_power_of_2_or_zero(notify_cfg.notify_off_multiplier().read()),
            "Notify offset multiplier must be a power of 2 or zero"
        );

        assert!(
            notify_cfg.cap().offset().read().is_multiple_of(16),
            "Notify offset must be 2 byte aligned"
        );

        assert!(
            notify_cfg.cap().length().read() >= 2,
            "Notify length must be at least 2"
        );

        let notify_bar = pci_device.get_or_initialize_bar(notify_cfg.cap().bar().read());

        // Intialize virtqueues
        // index 0
        common_cfg.queue_select().write(0);
        let mut receive_queue: VirtQueue<EXPECTED_QUEUE_SIZE> =
            VirtQueue::new(common_cfg.queue_size().read(), 0);
        // index 1
        common_cfg.queue_select().write(1);
        let mut transmit_queue: VirtQueue<EXPECTED_QUEUE_SIZE> =
            VirtQueue::new(common_cfg.queue_size().read(), 1);

        assert!(
            notify_cfg.cap().length().read()
                >= common_cfg.queue_notify_off().read() as u32
                    * notify_cfg.notify_off_multiplier().read()
                    + 2,
            "Notify length must be at least the notify offset"
        );

        let transmit_notify: MMIO<u16> = MMIO::new(
            notify_bar.cpu_address
                + notify_cfg.cap().offset().read() as usize
                + common_cfg.queue_notify_off().read() as usize
                    * notify_cfg.notify_off_multiplier().read() as usize,
        );

        transmit_queue.set_notify(transmit_notify);

        common_cfg.queue_select().write(0);
        common_cfg
            .queue_desc()
            .write(receive_queue.descriptor_area_physical_address());
        common_cfg
            .queue_driver()
            .write(receive_queue.driver_area_physical_address());
        common_cfg
            .queue_device()
            .write(receive_queue.device_area_physical_address());
        common_cfg.queue_enable().write(1);

        common_cfg.queue_select().write(1);
        common_cfg
            .queue_desc()
            .write(transmit_queue.descriptor_area_physical_address());
        common_cfg
            .queue_driver()
            .write(transmit_queue.driver_area_physical_address());
        common_cfg
            .queue_device()
            .write(transmit_queue.device_area_physical_address());
        common_cfg.queue_enable().write(1);

        device_status |= DEVICE_STATUS_DRIVER_OK;

        assert!(
            device_status.read() & DEVICE_STATUS_FAILED == 0,
            "Device failed"
        );

        assert!(
            device_status.read() & DEVICE_STATUS_DRIVER_OK != 0,
            "Device driver not ok"
        );

        debug!("Device initialized: {:#x?}", device_status);

        // Get net configuration
        let net_cfg_cap = virtio_capabilities
            .iter_mut()
            .find(|cap| cap.cfg_type().read() == VIRTIO_PCI_CAP_DEVICE_CFG)
            .ok_or("Device configuration capability not found")?;

        debug!("Device configuration capability found at {:?}", net_cfg_cap);

        let net_config_bar = pci_device.get_or_initialize_bar(net_cfg_cap.bar().read());

        let net_cfg: MMIO<virtio_net_config> =
            MMIO::new(net_config_bar.cpu_address + net_cfg_cap.offset().read() as usize);

        debug!("Net config: {:#x?}", net_cfg);

        // Fill receive buffers
        for _ in 0..EXPECTED_QUEUE_SIZE {
            let receive_buffer = vec![0xffu8; 1526];
            receive_queue
                .put_buffer(receive_buffer, BufferDirection::DeviceWritable)
                .expect("Receive buffer must be insertable to the queue");
        }

        let mac_address = net_cfg.mac().read();

        info!(
            "Successfully initialized network device at {:p} with mac {}",
            *pci_device.configuration_space(),
            mac_address
        );

        Ok(Self {
            device: pci_device,
            common_cfg,
            net_cfg,
            notify_cfg,
            mac_address,
            receive_queue,
            transmit_queue,
        })
    }

    pub fn receive_packets(&mut self) -> Vec<Vec<u8>> {
        let new_receive_buffers = self.receive_queue.receive_buffer();
        let mut received_packets = Vec::new();

        for receive_buffer in new_receive_buffers {
            let (net_hdr, data_bytes) = receive_buffer.buffer.split_as::<virtio_net_hdr>();

            assert!(net_hdr.gso_type == VIRTIO_NET_HDR_GSO_NONE);
            assert!(net_hdr.flags == 0);

            let data = data_bytes.to_vec();
            received_packets.push(data);

            // Put buffer back into receive queue
            self.receive_queue
                .put_buffer(receive_buffer.buffer, BufferDirection::DeviceWritable)
                .expect("Receive buffer must be insertable into the queue.");
        }

        received_packets
    }

    pub fn send_packet(&mut self, data: Vec<u8>) -> Result<u16, QueueError> {
        // First free all already transmited packets
        debug!("Going to free all buffers which were used to send packets.");
        for transmitted_packet in self.transmit_queue.receive_buffer() {
            debug!("Transmitted packet: {:?}", transmitted_packet.index);
        }

        let header = virtio_net_hdr {
            flags: 0,
            gso_type: VIRTIO_NET_HDR_GSO_NONE,
            hdr_len: 0,
            gso_size: 0,
            csum_start: 0,
            csum_offset: 0,
            num_buffers: 0,
        };

        let data = [header.as_slice(), data.as_slice()].concat();
        let index = self
            .transmit_queue
            .put_buffer(data, BufferDirection::DriverWritable);

        // Notify device
        self.transmit_queue.notify();

        index
    }

    pub fn get_mac_address(&self) -> MacAddress {
        self.mac_address
    }
}

impl Drop for NetworkDevice {
    fn drop(&mut self) {
        info!("Reset network device becuase of drop");
        self.common_cfg.device_status().write(0x0);
    }
}

mmio_struct! {
    #[repr(C)]
    struct virtio_pci_common_cfg {
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
    }
}

mmio_struct! {
    #[repr(C)]
    struct virtio_net_config {
        mac: crate::net::mac::MacAddress,
        status: u16,
        max_virtqueue_pairs: u16,
        mtu: u16,
        speed: u32,
        duplex: u8,
        rss_max_key_size: u8,
        rss_max_indirection_table_length: u16,
        supported_hash_types: u32,
    }
}

const VIRTIO_NET_HDR_GSO_NONE: u8 = 0;

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug)]
struct virtio_net_hdr {
    flags: u8,
    gso_type: u8,
    hdr_len: u16,
    gso_size: u16,
    csum_start: u16,
    csum_offset: u16,
    num_buffers: u16,
    // hash_value: u32,
    // hash_report: u16,
    // padding_reserved: u16,
}

static_assert_size!(virtio_net_hdr, 12);

impl ByteInterpretable for virtio_net_hdr {}

mmio_struct! {
    #[repr(C)]
    struct virtio_pci_notify_cap {
        cap: crate::drivers::virtio::capability::virtio_pci_cap,
        notify_off_multiplier: u32,
    }
}
