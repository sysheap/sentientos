use alloc::vec::Vec;
use headers::errno::Errno;

use crate::{
    drivers::virtio::{
        capability::{
            DEVICE_STATUS_ACKNOWLEDGE, DEVICE_STATUS_DRIVER, DEVICE_STATUS_DRIVER_OK,
            DEVICE_STATUS_FAILED, DEVICE_STATUS_FEATURES_OK, VIRTIO_DEVICE_ID, VIRTIO_F_VERSION_1,
            VIRTIO_PCI_CAP_COMMON_CFG, VIRTIO_PCI_CAP_DEVICE_CFG, VIRTIO_PCI_CAP_NOTIFY_CFG,
            VIRTIO_VENDOR_ID, VIRTIO_VENDOR_SPECIFIC_CAPABILITY_ID, virtio_pci_cap,
            virtio_pci_common_cfg, virtio_pci_notify_cap,
        },
        virtqueue::{BufferDirection, VirtQueue},
    },
    info,
    klibc::{
        MMIO, Spinlock,
        util::{ByteInterpretable, is_power_of_2_or_zero},
    },
    mmio_struct,
    pci::PCIDevice,
};

const EXPECTED_QUEUE_SIZE: usize = 0x100;
const SECTOR_SIZE: usize = 512;
const VIRTIO_BLOCK_SUBSYSTEM_ID: u16 = 2;

const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;
const VIRTIO_BLK_S_OK: u8 = 0;

#[repr(C)]
struct VirtioBlkReqHeader {
    request_type: u32,
    reserved: u32,
    sector: u64,
}

impl ByteInterpretable for VirtioBlkReqHeader {}

mmio_struct! {
    #[repr(C)]
    struct virtio_blk_config {
        capacity: u64,
    }
}

#[allow(dead_code)]
pub struct BlockDevice {
    device: PCIDevice,
    common_cfg: MMIO<virtio_pci_common_cfg>,
    blk_cfg: MMIO<virtio_blk_config>,
    request_queue: VirtQueue<EXPECTED_QUEUE_SIZE>,
    capacity_sectors: u64,
}

static BLOCK_DEVICE: Spinlock<Option<BlockDevice>> = Spinlock::new(None);

pub fn assign_block_device(device: BlockDevice) {
    *BLOCK_DEVICE.lock() = Some(device);
}

pub fn capacity() -> u64 {
    BLOCK_DEVICE
        .lock()
        .as_ref()
        .map_or(0, |d| d.capacity_bytes())
}

pub fn read(offset: usize, buf: &mut [u8]) -> Result<usize, Errno> {
    let mut guard = BLOCK_DEVICE.lock();
    let dev = guard.as_mut().ok_or(Errno::ENODEV)?;

    #[allow(clippy::cast_possible_truncation)]
    let cap = dev.capacity_bytes() as usize;
    if offset >= cap {
        return Ok(0);
    }
    let read_len = core::cmp::min(buf.len(), cap - offset);
    if read_len == 0 {
        return Ok(0);
    }

    let start_sector = offset / SECTOR_SIZE;
    let offset_in_first_sector = offset % SECTOR_SIZE;
    let end = offset + read_len;
    let end_sector = end.div_ceil(SECTOR_SIZE);
    let num_sectors = end_sector - start_sector;

    let mut sector_buf = vec![0u8; num_sectors * SECTOR_SIZE];
    dev.read_sectors(
        u64::try_from(start_sector).expect("sector fits in u64"),
        &mut sector_buf,
    );

    buf[..read_len]
        .copy_from_slice(&sector_buf[offset_in_first_sector..offset_in_first_sector + read_len]);
    Ok(read_len)
}

pub fn write(offset: usize, data: &[u8]) -> Result<usize, Errno> {
    let mut guard = BLOCK_DEVICE.lock();
    let dev = guard.as_mut().ok_or(Errno::ENODEV)?;

    #[allow(clippy::cast_possible_truncation)]
    let cap = dev.capacity_bytes() as usize;
    if offset >= cap {
        return Ok(0);
    }
    let write_len = core::cmp::min(data.len(), cap - offset);
    if write_len == 0 {
        return Ok(0);
    }

    let start_sector = offset / SECTOR_SIZE;
    let offset_in_first_sector = offset % SECTOR_SIZE;
    let end = offset + write_len;
    let end_sector = end.div_ceil(SECTOR_SIZE);
    let num_sectors = end_sector - start_sector;

    // If not sector-aligned, read-modify-write
    let mut sector_buf = vec![0u8; num_sectors * SECTOR_SIZE];
    if offset_in_first_sector != 0 || !end.is_multiple_of(SECTOR_SIZE) {
        dev.read_sectors(
            u64::try_from(start_sector).expect("sector fits in u64"),
            &mut sector_buf,
        );
    }

    sector_buf[offset_in_first_sector..offset_in_first_sector + write_len]
        .copy_from_slice(&data[..write_len]);

    dev.write_sectors(
        u64::try_from(start_sector).expect("sector fits in u64"),
        &sector_buf,
    );

    Ok(write_len)
}

impl BlockDevice {
    pub fn is_virtio_block(device: &PCIDevice) -> bool {
        let cs = device.configuration_space();
        cs.vendor_id().read() == VIRTIO_VENDOR_ID
            && VIRTIO_DEVICE_ID.contains(&cs.device_id().read())
            && cs.subsystem_id().read() == VIRTIO_BLOCK_SUBSYSTEM_ID
    }

    pub fn initialize(mut pci_device: PCIDevice) -> Result<Self, &'static str> {
        let capabilities = pci_device.capabilities();
        let mut virtio_capabilities: Vec<MMIO<virtio_pci_cap>> = capabilities
            .filter(|cap| cap.id().read() == VIRTIO_VENDOR_SPECIFIC_CAPABILITY_ID)
            // SAFETY: VirtIO vendor-specific capabilities have the virtio_pci_cap
            // layout per the VirtIO spec.
            .map(|cap| unsafe { cap.new_type::<virtio_pci_cap>() })
            .collect();

        let common_cfg_cap = virtio_capabilities
            .iter()
            .find(|cap| cap.cfg_type().read() == VIRTIO_PCI_CAP_COMMON_CFG)
            .ok_or("Common configuration capability not found")?;

        let config_bar = pci_device.get_or_initialize_bar(common_cfg_cap.bar().read());
        let common_cfg: MMIO<virtio_pci_common_cfg> = MMIO::new(
            (config_bar.cpu_address + common_cfg_cap.offset().read() as usize).as_usize(),
        );

        // Reset and acknowledge
        common_cfg.device_status().write(0x0);
        #[allow(clippy::while_immutable_condition)]
        while common_cfg.device_status().read() != 0x0 {}

        let mut device_status = common_cfg.device_status();
        device_status |= DEVICE_STATUS_ACKNOWLEDGE;
        assert!(
            device_status.read() & DEVICE_STATUS_FAILED == 0,
            "Device failed"
        );
        device_status |= DEVICE_STATUS_DRIVER;
        assert!(
            device_status.read() & DEVICE_STATUS_FAILED == 0,
            "Device failed"
        );

        // Negotiate features (only VIRTIO_F_VERSION_1)
        common_cfg.device_feature_select().write(0);
        let mut device_features = common_cfg.device_feature().read() as u64;
        common_cfg.device_feature_select().write(1);
        device_features |= (common_cfg.device_feature().read() as u64) << 32;

        assert!(
            device_features & VIRTIO_F_VERSION_1 != 0,
            "Virtio version 1 not supported"
        );

        let wanted_features: u64 = VIRTIO_F_VERSION_1;

        common_cfg.driver_feature_select().write(0);
        common_cfg
            .driver_feature()
            .write(u32::try_from(wanted_features & 0xFFFF_FFFF).expect("masked to 32 bits"));
        common_cfg.driver_feature_select().write(1);
        common_cfg
            .driver_feature()
            .write(u32::try_from(wanted_features >> 32).expect("high 32 bits fit in u32"));

        device_status |= DEVICE_STATUS_FEATURES_OK;
        assert!(
            device_status.read() & DEVICE_STATUS_FAILED == 0,
            "Device failed"
        );
        assert!(
            device_status.read() & DEVICE_STATUS_FEATURES_OK != 0,
            "Device features not ok"
        );

        // Setup notification
        let notify_cfg_cap = virtio_capabilities
            .iter()
            .find(|cap| cap.cfg_type().read() == VIRTIO_PCI_CAP_NOTIFY_CFG)
            .ok_or("Notification capability not found")?;
        // SAFETY: The notify capability extends virtio_pci_cap with an
        // additional notify_off_multiplier field per the VirtIO spec.
        let notify_cfg = unsafe { notify_cfg_cap.new_type::<virtio_pci_notify_cap>() };

        assert!(
            is_power_of_2_or_zero(notify_cfg.notify_off_multiplier().read()),
            "Notify offset multiplier must be a power of 2 or zero"
        );

        let notify_bar = pci_device.get_or_initialize_bar(notify_cfg.cap().bar().read());

        // Setup single request queue at index 0
        common_cfg.queue_select().write(0);
        let mut request_queue: VirtQueue<EXPECTED_QUEUE_SIZE> =
            VirtQueue::new(common_cfg.queue_size().read(), 0);

        let notify_mmio: MMIO<u16> = MMIO::new(
            notify_bar.cpu_address.as_usize()
                + notify_cfg.cap().offset().read() as usize
                + common_cfg.queue_notify_off().read() as usize
                    * notify_cfg.notify_off_multiplier().read() as usize,
        );
        request_queue.set_notify(notify_mmio);

        // Configure queue on device
        common_cfg.queue_select().write(0);
        common_cfg
            .queue_desc()
            .write(request_queue.descriptor_area_physical_address());
        common_cfg
            .queue_driver()
            .write(request_queue.driver_area_physical_address());
        common_cfg
            .queue_device()
            .write(request_queue.device_area_physical_address());
        common_cfg.queue_enable().write(1);

        // Read device config (capacity)
        let blk_cfg_cap = virtio_capabilities
            .iter_mut()
            .find(|cap| cap.cfg_type().read() == VIRTIO_PCI_CAP_DEVICE_CFG)
            .ok_or("Device configuration capability not found")?;

        let blk_config_bar = pci_device.get_or_initialize_bar(blk_cfg_cap.bar().read());
        let blk_cfg: MMIO<virtio_blk_config> = MMIO::new(
            (blk_config_bar.cpu_address + blk_cfg_cap.offset().read() as usize).as_usize(),
        );

        let capacity_sectors = blk_cfg.capacity().read();

        // Mark driver ready
        device_status |= DEVICE_STATUS_DRIVER_OK;
        assert!(
            device_status.read() & DEVICE_STATUS_FAILED == 0,
            "Device failed"
        );

        // Enable bus master for DMA
        pci_device
            .configuration_space_mut()
            .set_command_register_bits(crate::pci::command_register::BUS_MASTER);

        info!(
            "Successfully initialized block device: {} sectors ({} bytes)",
            capacity_sectors,
            capacity_sectors * u64::try_from(SECTOR_SIZE).expect("fits")
        );

        Ok(Self {
            device: pci_device,
            common_cfg,
            blk_cfg,
            request_queue,
            capacity_sectors,
        })
    }

    fn capacity_bytes(&self) -> u64 {
        self.capacity_sectors * u64::try_from(SECTOR_SIZE).expect("fits")
    }

    fn read_sectors(&mut self, start_sector: u64, buf: &mut [u8]) {
        assert!(
            buf.len().is_multiple_of(SECTOR_SIZE),
            "Buffer must be sector-aligned"
        );
        let num_sectors = buf.len() / SECTOR_SIZE;
        assert!(
            start_sector + u64::try_from(num_sectors).expect("fits") <= self.capacity_sectors,
            "Read beyond device capacity"
        );

        let header = VirtioBlkReqHeader {
            request_type: VIRTIO_BLK_T_IN,
            reserved: 0,
            sector: start_sector,
        };

        let header_buf = header.as_slice().to_vec();
        let data_buf = vec![0u8; buf.len()];
        let status_buf = vec![0u8; 1];

        let chain = vec![
            (header_buf, BufferDirection::DriverWritable),
            (data_buf, BufferDirection::DeviceWritable),
            (status_buf, BufferDirection::DeviceWritable),
        ];

        self.request_queue
            .put_buffer_chain(chain)
            .expect("Must be able to submit block request");
        self.request_queue.notify();

        // Spin-wait for completion
        loop {
            let completed = self.request_queue.receive_buffer();
            if !completed.is_empty() {
                assert!(completed.len() == 1, "Expected single completion");
                let result = completed.into_iter().next().expect("checked");
                assert!(result.buffers.len() == 3, "Expected 3-descriptor chain");
                let status = result.buffers[2][0];
                assert!(
                    status == VIRTIO_BLK_S_OK,
                    "Block read failed with status {}",
                    status
                );
                buf.copy_from_slice(&result.buffers[1]);
                return;
            }
            core::hint::spin_loop();
        }
    }

    fn write_sectors(&mut self, start_sector: u64, data: &[u8]) {
        assert!(
            data.len().is_multiple_of(SECTOR_SIZE),
            "Data must be sector-aligned"
        );
        let num_sectors = data.len() / SECTOR_SIZE;
        assert!(
            start_sector + u64::try_from(num_sectors).expect("fits") <= self.capacity_sectors,
            "Write beyond device capacity"
        );

        let header = VirtioBlkReqHeader {
            request_type: VIRTIO_BLK_T_OUT,
            reserved: 0,
            sector: start_sector,
        };

        let header_buf = header.as_slice().to_vec();
        let data_buf = data.to_vec();
        let status_buf = vec![0u8; 1];

        let chain = vec![
            (header_buf, BufferDirection::DriverWritable),
            (data_buf, BufferDirection::DriverWritable),
            (status_buf, BufferDirection::DeviceWritable),
        ];

        self.request_queue
            .put_buffer_chain(chain)
            .expect("Must be able to submit block request");
        self.request_queue.notify();

        // Spin-wait for completion
        loop {
            let completed = self.request_queue.receive_buffer();
            if !completed.is_empty() {
                assert!(completed.len() == 1, "Expected single completion");
                let result = completed.into_iter().next().expect("checked");
                assert!(result.buffers.len() == 3, "Expected 3-descriptor chain");
                let status = result.buffers[2][0];
                assert!(
                    status == VIRTIO_BLK_S_OK,
                    "Block write failed with status {}",
                    status
                );
                return;
            }
            core::hint::spin_loop();
        }
    }
}

impl Drop for BlockDevice {
    fn drop(&mut self) {
        info!("Reset block device because of drop");
        self.common_cfg.device_status().write(0x0);
    }
}
