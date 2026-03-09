use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use headers::errno::Errno;

use crate::{
    drivers::{
        bochs_display,
        virtio::{block, rng},
    },
    io::tty_device::{TtyDevice, console_tty},
    klibc::{MMIO, Spinlock},
};

use super::vfs::{DirEntry, NodeType, VfsNode, VfsNodeRef, alloc_ino};

struct DevNull {
    ino: u64,
}

impl VfsNode for DevNull {
    fn node_type(&self) -> NodeType {
        NodeType::File
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    fn size(&self) -> usize {
        0
    }

    fn read(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize, Errno> {
        Ok(0)
    }

    fn write(&self, _offset: usize, data: &[u8]) -> Result<usize, Errno> {
        Ok(data.len())
    }

    fn truncate(&self) -> Result<(), Errno> {
        Ok(())
    }
}

struct DevZero {
    ino: u64,
}

impl VfsNode for DevZero {
    fn node_type(&self) -> NodeType {
        NodeType::File
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    fn size(&self) -> usize {
        0
    }

    fn read(&self, _offset: usize, buf: &mut [u8]) -> Result<usize, Errno> {
        buf.fill(0);
        Ok(buf.len())
    }

    fn write(&self, _offset: usize, data: &[u8]) -> Result<usize, Errno> {
        Ok(data.len())
    }

    fn truncate(&self) -> Result<(), Errno> {
        Ok(())
    }
}

struct DevBlock {
    ino: u64,
    index: usize,
}

impl VfsNode for DevBlock {
    fn node_type(&self) -> NodeType {
        NodeType::File
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    #[allow(clippy::cast_possible_truncation)]
    fn size(&self) -> usize {
        block::capacity(self.index) as usize
    }

    fn truncate(&self) -> Result<(), Errno> {
        Err(Errno::EINVAL)
    }

    fn block_device_index(&self) -> Option<usize> {
        Some(self.index)
    }
}

struct DevRandom {
    ino: u64,
}

impl VfsNode for DevRandom {
    fn node_type(&self) -> NodeType {
        NodeType::File
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    fn size(&self) -> usize {
        0
    }

    fn read(&self, _offset: usize, buf: &mut [u8]) -> Result<usize, Errno> {
        rng::read_random(buf);
        Ok(buf.len())
    }

    fn write(&self, _offset: usize, data: &[u8]) -> Result<usize, Errno> {
        Ok(data.len())
    }

    fn truncate(&self) -> Result<(), Errno> {
        Ok(())
    }
}

struct DevConsole {
    ino: u64,
    device: TtyDevice,
}

impl VfsNode for DevConsole {
    fn node_type(&self) -> NodeType {
        NodeType::File
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    fn size(&self) -> usize {
        0
    }

    fn read(&self, _offset: usize, buf: &mut [u8]) -> Result<usize, Errno> {
        let mut dev = self.device.lock();
        let data = dev.get_input(buf.len());
        if data.is_empty() {
            return Err(Errno::EAGAIN);
        }
        buf[..data.len()].copy_from_slice(&data);
        Ok(data.len())
    }

    fn write(&self, _offset: usize, data: &[u8]) -> Result<usize, Errno> {
        let processed = self.device.lock().process_output(data);
        let mut uart = crate::io::uart::QEMU_UART.lock();
        for &b in &processed {
            uart.write_byte(b);
        }
        Ok(data.len())
    }

    fn truncate(&self) -> Result<(), Errno> {
        Ok(())
    }
}

struct DevfsDir {
    ino: u64,
    entries: Spinlock<BTreeMap<String, VfsNodeRef>>,
}

impl VfsNode for DevfsDir {
    fn node_type(&self) -> NodeType {
        NodeType::Directory
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    fn size(&self) -> usize {
        0
    }

    fn lookup(&self, name: &str) -> Result<VfsNodeRef, Errno> {
        self.entries.lock().get(name).cloned().ok_or(Errno::ENOENT)
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, Errno> {
        Ok(self
            .entries
            .lock()
            .iter()
            .map(|(name, node)| DirEntry {
                name: name.clone(),
                ino: node.ino(),
                node_type: node.node_type(),
            })
            .collect())
    }
}

static DEVFS: Spinlock<Option<Arc<DevfsDir>>> = Spinlock::new(None);

pub(super) fn new() -> VfsNodeRef {
    let mut entries = BTreeMap::new();
    entries.insert(
        String::from("null"),
        Arc::new(DevNull { ino: alloc_ino() }) as VfsNodeRef,
    );
    entries.insert(
        String::from("zero"),
        Arc::new(DevZero { ino: alloc_ino() }) as VfsNodeRef,
    );
    entries.insert(
        String::from("console"),
        Arc::new(DevConsole {
            ino: alloc_ino(),
            device: console_tty().clone(),
        }) as VfsNodeRef,
    );

    let dir = Arc::new(DevfsDir {
        ino: alloc_ino(),
        entries: Spinlock::new(entries),
    });
    *DEVFS.lock() = Some(dir.clone());
    dir
}

pub fn register_random_device() {
    let node: VfsNodeRef = Arc::new(DevRandom { ino: alloc_ino() });
    let dir = DEVFS
        .lock()
        .clone()
        .expect("devfs must be initialized before registering devices");
    dir.entries.lock().insert(String::from("random"), node);
}

struct DevFramebuffer {
    ino: u64,
}

impl VfsNode for DevFramebuffer {
    fn node_type(&self) -> NodeType {
        NodeType::File
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    fn size(&self) -> usize {
        bochs_display::FB_SIZE
    }

    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, Errno> {
        let base = bochs_display::fb_base();
        if base == 0 {
            return Err(Errno::ENODEV);
        }
        let end = offset.saturating_add(buf.len()).min(bochs_display::FB_SIZE);
        if offset >= end {
            return Ok(0);
        }
        let len = end - offset;
        for (i, byte) in buf[..len].iter_mut().enumerate() {
            let mmio: MMIO<u8> = MMIO::new(base + offset + i);
            *byte = mmio.read();
        }
        Ok(len)
    }

    fn write(&self, offset: usize, data: &[u8]) -> Result<usize, Errno> {
        let base = bochs_display::fb_base();
        if base == 0 {
            return Err(Errno::ENODEV);
        }
        let end = offset
            .saturating_add(data.len())
            .min(bochs_display::FB_SIZE);
        if offset >= end {
            return Ok(0);
        }
        let len = end - offset;
        for (i, &byte) in data[..len].iter().enumerate() {
            let mut mmio: MMIO<u8> = MMIO::new(base + offset + i);
            mmio.write(byte);
        }
        Ok(len)
    }

    fn truncate(&self) -> Result<(), Errno> {
        Ok(())
    }
}

pub fn register_framebuffer_device() {
    let node: VfsNodeRef = Arc::new(DevFramebuffer { ino: alloc_ino() });
    let dir = DEVFS
        .lock()
        .clone()
        .expect("devfs must be initialized before registering devices");
    dir.entries.lock().insert(String::from("fb0"), node);
}

pub fn register_block_device(index: usize) {
    assert!(index < 26, "block device index must be < 26 (a-z)");
    #[allow(clippy::cast_possible_truncation)]
    let suffix = (b'a' + index as u8) as char;
    let name = alloc::format!("vd{suffix}");
    let node: VfsNodeRef = Arc::new(DevBlock {
        ino: alloc_ino(),
        index,
    });
    let dir = DEVFS
        .lock()
        .clone()
        .expect("devfs must be initialized before registering devices");
    dir.entries.lock().insert(name, node);
}
