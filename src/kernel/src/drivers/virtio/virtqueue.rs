use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::{cpu, info, klibc::MMIO};

/// A virtio queue.
/// Using Box to prevent content from being moved.
pub struct VirtQueue<const QUEUE_SIZE: usize> {
    descriptor_area: Box<[virtq_desc; QUEUE_SIZE]>,
    free_descriptor_indices: Vec<u16>,
    outstanding_buffers: BTreeMap<u16, DeconstructedVec>,
    last_used_ring_index: u16,
    driver_area: Box<virtq_avail<QUEUE_SIZE>>,
    device_area: Box<virtq_used<QUEUE_SIZE>>,
    queue_index: u16,
    notify: Option<MMIO<u16>>,
}

#[allow(dead_code)]
struct DeconstructedVec {
    ptr: *mut u8,
    length: usize,
    capacity: usize,
}

impl DeconstructedVec {
    fn from_vec(vec: Vec<u8>) -> Self {
        let (ptr, length, capacity) = vec.into_raw_parts();
        Self {
            ptr,
            length,
            capacity,
        }
    }

    fn into_vec_with_len(self, length: usize) -> Vec<u8> {
        assert!(
            length <= self.capacity,
            "Length must be smaller or equal capacity"
        );
        unsafe { Vec::from_raw_parts(self.ptr, length, self.capacity) }
    }
}

pub enum BufferDirection {
    DriverWritable,
    DeviceWritable,
}

#[derive(Debug)]
pub enum QueueError {
    NoFreeDescriptors,
}

impl<const QUEUE_SIZE: usize> VirtQueue<QUEUE_SIZE> {
    pub fn new(queue_size: u16, queue_index: u16) -> Self {
        assert!(queue_size == QUEUE_SIZE as u16, "Queue size must be equal");
        assert!(queue_size % 2 == 0, "Queue size must be a power of 2");
        let queue = VirtQueue {
            descriptor_area: Box::new(core::array::from_fn(|_| virtq_desc::default())),
            free_descriptor_indices: (0..queue_size).collect(),
            outstanding_buffers: BTreeMap::new(),
            last_used_ring_index: 0,
            driver_area: Box::<virtq_avail<QUEUE_SIZE>>::default(),
            device_area: Box::<virtq_used<QUEUE_SIZE>>::default(),
            queue_index,
            notify: None,
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

    pub fn set_notify(&mut self, notify: MMIO<u16>) {
        self.notify = Some(notify);
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

    /// Put a buffer into the virtque.
    /// Returns the id of the descriptor if the request was successful.
    /// Returns the original request data in case the request was errornous.
    pub fn put_buffer(
        &mut self,
        buffer: Vec<u8>,
        direction: BufferDirection,
    ) -> Result<u16, QueueError> {
        let free_descriptor_index = self
            .free_descriptor_indices
            .pop()
            .ok_or(QueueError::NoFreeDescriptors)?;
        let descriptor = &mut self.descriptor_area[free_descriptor_index as usize];
        descriptor.addr = buffer.as_ptr() as u64;
        descriptor.len = buffer.len() as u32;
        descriptor.flags = match direction {
            BufferDirection::DeviceWritable => VIRTQ_DESC_F_WRITE,
            BufferDirection::DriverWritable => 0,
        };
        descriptor.next = 0;

        // Set available ring
        // avail->ring[avail->idx % qsz] = head;
        self.driver_area.ring[self.driver_area.idx as usize % QUEUE_SIZE] = free_descriptor_index;

        cpu::memory_fence();

        self.driver_area.idx = self.driver_area.idx.wrapping_add(1);

        cpu::memory_fence();

        let insert_result = self
            .outstanding_buffers
            .insert(free_descriptor_index, DeconstructedVec::from_vec(buffer))
            .is_none();

        assert!(
            insert_result,
            "Outstanding buffers is not allowed to contain this index"
        );

        Ok(free_descriptor_index)
    }

    pub fn receive_buffer(&mut self) -> Vec<UsedBuffer> {
        cpu::memory_fence();
        // Prevent re/reading the hardware. Only tackle the current amount of buffers.
        let current_device_index = self.device_area.idx;
        if self.last_used_ring_index == current_device_index {
            return Vec::new();
        }
        info!("Current device index: {:#x?}", current_device_index);
        let mut return_buffers: Vec<UsedBuffer> = Vec::new();
        while self.last_used_ring_index != current_device_index {
            info!("last used ring index: {:#x?}", self.last_used_ring_index);
            let result_descriptor =
                &mut self.device_area.ring[self.last_used_ring_index as usize % QUEUE_SIZE];
            let descriptor_entry = &mut self.descriptor_area[result_descriptor.id as usize];
            info!("Received packet from descriptor {:#x?}", descriptor_entry);
            assert!(
                descriptor_entry.flags == VIRTQ_DESC_F_WRITE,
                "Only the \"device writable\" flag is allowed for the descriptor entry"
            );
            info!("Result descriptor {:#x?}", result_descriptor);
            let index = result_descriptor.id as u16;
            let buffer = self
                .outstanding_buffers
                .remove(&index)
                .expect("There must be an outstanding buffer for this id")
                .into_vec_with_len(result_descriptor.len as usize);
            return_buffers.push(UsedBuffer { index, buffer });
            descriptor_entry.addr = 0;
            descriptor_entry.len = 0;
            self.free_descriptor_indices.push(index);
            self.last_used_ring_index = self.last_used_ring_index.wrapping_add(1);
        }
        return_buffers
    }

    pub fn notify(&mut self) {
        if let Some(notify) = &mut self.notify {
            **notify = self.queue_index;
        }
    }
}

#[derive(Debug)]
pub struct UsedBuffer {
    pub index: u16,
    pub buffer: Vec<u8>,
}

/* This marks a buffer as continuing via the next field. */
#[allow(dead_code)]
const VIRTQ_DESC_F_NEXT: u16 = 1;
/* This marks a buffer as device write-only (otherwise device read-only). */
const VIRTQ_DESC_F_WRITE: u16 = 2;
/* This means the buffer contains a list of buffer descriptors. */
#[allow(dead_code)]
const VIRTQ_DESC_F_INDIRECT: u16 = 4;

#[allow(non_camel_case_types)]
#[repr(C, align(16))]
#[derive(Default, Debug)]
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
#[derive(Default, Debug)]
struct virtq_used_elem {
    id: u32, /* Index of start of used descriptor chain. */
    len: u32, /*
              * The number of bytes written into the device writable portion of
              * the buffer described by the descriptor chain.
              */
}
