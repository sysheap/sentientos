use core::mem;

use crate::{
    assert::static_assert_size,
    debug,
    drivers::virtio::{
        capability::{VirtioPciCap, VIRTIO_PCI_CAP_COMMON_CFG, VIRTIO_PCI_CAP_DEVICE_CFG},
        virtqueue::{BufferDirection, VirtQueue},
    },
    info,
    klibc::MMIO,
    pci::{GeneralDevicePciHeader, PCIInformation},
};
use alloc::vec::Vec;

const EXPECTED_QUEUE_SIZE: usize = 0x100;

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
    common_cfg: MMIO<virtio_pci_commonf_cfg>,
    net_cfg: MMIO<virtio_net_config>,
    transmit_queue: VirtQueue<EXPECTED_QUEUE_SIZE>,
    receive_queue: VirtQueue<EXPECTED_QUEUE_SIZE>,
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

        debug!(
            "Common configuration capability found at {:?}",
            **common_cfg
        );

        let bar_index = common_cfg.bar();

        let config_bar = pci_device.initialize_bar(bar_index);

        let mut common_cfg: MMIO<virtio_pci_commonf_cfg> =
            unsafe { MMIO::new(config_bar.cpu_address + common_cfg.offset()) };

        debug!("Common config: {:#x?}", *common_cfg);

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

        // Intialize virtqueues
        // index 0
        common_cfg.queue_select = 0;
        let mut receive_queue: VirtQueue<EXPECTED_QUEUE_SIZE> =
            VirtQueue::new(common_cfg.queue_size);
        // index 1
        common_cfg.queue_select = 1;
        let transmit_queue: VirtQueue<EXPECTED_QUEUE_SIZE> = VirtQueue::new(common_cfg.queue_size);

        common_cfg.queue_select = 0;
        common_cfg.queue_desc = receive_queue.descriptor_area_physical_address() as u64;
        common_cfg.queue_driver = receive_queue.driver_area_physical_address() as u64;
        common_cfg.queue_device = receive_queue.device_area_physical_address() as u64;
        common_cfg.queue_enable = 1;

        common_cfg.queue_select = 1;
        common_cfg.queue_desc = transmit_queue.descriptor_area_physical_address() as u64;
        common_cfg.queue_driver = transmit_queue.driver_area_physical_address() as u64;
        common_cfg.queue_device = transmit_queue.device_area_physical_address() as u64;
        common_cfg.queue_enable = 1;

        common_cfg.device_status = common_cfg.device_status | DEVICE_STATUS_DRIVER_OK;

        assert!(
            common_cfg.device_status & DEVICE_STATUS_DRIVER_OK != 0,
            "Device driver not ok"
        );

        debug!("Device initialized: {:#x?}", common_cfg.device_status);

        // Get device configuration
        let net_cfg_cap = virtio_capabilities
            .iter()
            .find(|cap| cap.cfg_type() == VIRTIO_PCI_CAP_DEVICE_CFG)
            .ok_or("Device configuration capability not found")?;

        debug!(
            "Device configuration capability found at {:?}",
            **net_cfg_cap
        );

        let net_config_bar_index = net_cfg_cap.bar();

        // TODO: Remember which bar is already configured
        assert!(net_config_bar_index == bar_index);

        // let net_config_bar = pci_device.initialize_bar(net_config_bar_index);

        let net_cfg: MMIO<virtio_net_config> =
            unsafe { MMIO::new(config_bar.cpu_address + net_cfg_cap.offset()) };

        debug!("Net config: {:#x?}", *net_cfg);

        // Fill receive buffers
        for _ in 0..EXPECTED_QUEUE_SIZE {
            let receive_buffer = vec![0xffu8; 1526];
            receive_queue
                .put_buffer(receive_buffer, BufferDirection::DeviceWritable)
                .expect("Receive buffer must be insertable to the queue");
        }

        info!(
            "Successfully initialized network device at {:p}",
            pci_device
        );

        Ok(Self {
            device: pci_device,
            common_cfg,
            net_cfg,
            receive_queue,
            transmit_queue,
        })
    }

    pub fn receive_packets(&mut self) -> Vec<Vec<u8>> {
        let new_receive_buffers = self.receive_queue.receive_buffer();
        let mut received_packets = Vec::new();

        for receive_buffer in new_receive_buffers {
            let (header_bytes, data_bytes) = receive_buffer
                .buffer
                .split_at(mem::size_of::<virtio_net_hdr>());

            let net_hdr: &virtio_net_hdr = unsafe {
                assert!(header_bytes.len() == mem::size_of::<virtio_net_hdr>());
                let ptr: *const virtio_net_hdr = header_bytes.as_ptr() as *const virtio_net_hdr;
                assert!(ptr.is_aligned(), "net hdr must be aligned");
                &*ptr
            };

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
}

impl Drop for NetworkDevice {
    fn drop(&mut self) {
        info!("Reset network device becuase of drop");
        self.common_cfg.device_status = 0x0;
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
#[repr(C)]
struct virtio_pci_commonf_cfg {
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

#[allow(non_camel_case_types)]
#[derive(Debug)]
#[repr(C)]
struct virtio_net_config {
    mac: [u8; 6],
    status: u16,
    max_virtqueue_pairs: u16,
    mtu: u16,
    speed: u32,
    duplex: u8,
    rss_max_key_size: u8,
    rss_max_indirection_table_length: u16,
    supported_hash_types: u32,
}

const VIRTIO_NET_HDR_F_NEEDS_CSUM: u8 = 1;
const VIRTIO_NET_HDR_F_DATA_VALID: u8 = 2;
const VIRTIO_NET_HDR_F_RSC_INFO: u8 = 4;

const VIRTIO_NET_HDR_GSO_NONE: u8 = 0;
const VIRTIO_NET_HDR_GSO_TCPV4: u8 = 1;
const VIRTIO_NET_HDR_GSO_UDP: u8 = 3;
const VIRTIO_NET_HDR_GSO_TCPV6: u8 = 4;
const VIRTIO_NET_HDR_GSO_UDP_L4: u8 = 5;
const VIRTIO_NET_HDR_GSO_ECN: u8 = 0x80;

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
