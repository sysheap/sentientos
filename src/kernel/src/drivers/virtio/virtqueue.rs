use alloc::boxed::Box;

/// A virtio queue.
/// Using Box to prevent content from being moved.
pub struct VirtQueue<const QUEUE_SIZE: usize> {
    descriptor_area: Box<[virtq_desc; QUEUE_SIZE]>,
    driver_area: Box<virtq_avail<QUEUE_SIZE>>,
    device_area: Box<virtq_used<QUEUE_SIZE>>,
}

impl<const QUEUE_SIZE: usize> VirtQueue<QUEUE_SIZE> {
    pub fn new(queue_size: u16) -> Self {
        assert!(queue_size == QUEUE_SIZE as u16, "Queue size must be equal");
        assert!(queue_size % 2 == 0, "Queue size must be a power of 2");
        let queue = VirtQueue {
            descriptor_area: Box::new(core::array::from_fn(|_| virtq_desc::default())),
            driver_area: Box::new(virtq_avail::default()),
            device_area: Box::new(virtq_used::default()),
        };
        assert!(
            queue.descriptor_area_physical_address() % 16 == 0,
            "Descriptor area not aligned"
        );
        assert!(
            queue.driver_area_physical_address() % 2 == 0,
            "Driver area not aligned"
        );
        assert!(
            queue.device_area_physical_address() % 4 == 0,
            "Device area not aligned"
        );

        queue
    }

    pub fn descriptor_area_physical_address(&self) -> u64 {
        self.descriptor_area.as_ptr() as u64
    }

    pub fn driver_area_physical_address(&self) -> u64 {
        &*self.driver_area as *const _ as u64
    }

    pub fn device_area_physical_address(&self) -> u64 {
        &*self.device_area as *const _ as u64
    }
}

/* This marks a buffer as continuing via the next field. */
const VIRTQ_DESC_F_NEXT: u16 = 1;
/* This marks a buffer as device write-only (otherwise device read-only). */
const VIRTQ_DESC_F_WRITE: u16 = 2;
/* This means the buffer contains a list of buffer descriptors. */
const VIRTQ_DESC_F_INDIRECT: u16 = 4;

#[allow(non_camel_case_types)]
#[repr(C, align(16))]
#[derive(Default)]
struct virtq_desc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

const VIRTQ_AVAIL_F_NO_INTERRUPT: u16 = 1;

#[allow(non_camel_case_types)]
#[repr(C, align(2))]
struct virtq_avail<const QUEUE_SIZE: usize> {
    flags: u16,
    idx: u16,
    ring: [u16; QUEUE_SIZE],
    used_event: u16, /* Only if VIRTIO_F_EVENT_IDX */
}

impl<const QUEUE_SIZE: usize> Default for virtq_avail<QUEUE_SIZE> {
    fn default() -> Self {
        Self {
            flags: VIRTQ_AVAIL_F_NO_INTERRUPT, // Ignore interrupts for the beginning
            idx: 0,
            ring: [0; QUEUE_SIZE],
            used_event: Default::default(),
        }
    }
}

const VIRTQ_USED_F_NO_NOTIFY: u16 = 1;

#[allow(non_camel_case_types)]
#[repr(C, align(4))]
struct virtq_used<const QUEUE_SIZE: usize> {
    flags: u16,
    idx: u16,
    ring: [virtq_used_elem; QUEUE_SIZE],
    avail_event: u16, /* Only if VIRTIO_F_EVENT_IDX */
}

impl<const QUEUE_SIZE: usize> Default for virtq_used<QUEUE_SIZE> {
    fn default() -> Self {
        Self {
            flags: VIRTQ_USED_F_NO_NOTIFY,
            idx: 0,
            ring: core::array::from_fn(|_| virtq_used_elem::default()),
            avail_event: Default::default(),
        }
    }
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Default)]
struct virtq_used_elem {
    id: u32, /* Index of start of used descriptor chain. */
    len: u32, /*
              * The number of bytes written into the device writable portion of
              * the buffer described by the descriptor chain.
              */
}
