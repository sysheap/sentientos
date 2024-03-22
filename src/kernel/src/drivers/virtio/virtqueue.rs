use alloc::boxed::Box;

pub struct VirtQueue<const QUEUE_SIZE: usize> {
    descriptor_area: Box<[virtq_desc; QUEUE_SIZE]>,
    driver_area: Box<virtq_avail<QUEUE_SIZE>>,
    device_area: Box<virtq_used<QUEUE_SIZE>>,
}

impl<const QUEUE_SIZE: usize> VirtQueue<QUEUE_SIZE> {
    pub fn new(queue_size: u16) -> Self {
        assert!(queue_size == QUEUE_SIZE as u16, "Queue size must be equal");
        VirtQueue {
            descriptor_area: Box::new(core::array::from_fn(|_| virtq_desc::default())),
            driver_area: Box::new(virtq_avail::default()),
            device_area: Box::new(virtq_used::default()),
        }
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

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;
const VIRTQ_DESC_F_INDIRECT: u16 = 4;

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Default)]
struct virtq_desc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

const VIRTQ_AVAIL_F_NO_INTERRUPT: u16 = 1;

#[allow(non_camel_case_types)]
#[repr(C)]
struct virtq_avail<const QUEUE_SIZE: usize> {
    flags: u16,
    idx: u16,
    ring: [u16; QUEUE_SIZE],
    used_event: u16, /* Only if VIRTIO_F_EVENT_IDX */
}

impl<const QUEUE_SIZE: usize> Default for virtq_avail<QUEUE_SIZE> {
    fn default() -> Self {
        Self {
            flags: Default::default(),
            idx: Default::default(),
            ring: [0; QUEUE_SIZE],
            used_event: Default::default(),
        }
    }
}

const VIRTQ_USED_F_NO_NOTIFY: u16 = 1;

#[allow(non_camel_case_types)]
#[repr(C)]
struct virtq_used<const QUEUE_SIZE: usize> {
    flags: u16,
    idx: u16,
    ring: [virtq_used_elem; QUEUE_SIZE],
    avail_event: u16, /* Only if VIRTIO_F_EVENT_IDX */
}

impl<const QUEUE_SIZE: usize> Default for virtq_used<QUEUE_SIZE> {
    fn default() -> Self {
        Self {
            flags: Default::default(),
            idx: Default::default(),
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
