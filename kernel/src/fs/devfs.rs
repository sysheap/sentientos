use alloc::sync::Arc;
use headers::errno::Errno;

use crate::drivers::virtio::block;

use super::vfs::{NodeType, StaticDir, VfsNode, alloc_ino};

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

struct DevVda {
    ino: u64,
}

impl VfsNode for DevVda {
    fn node_type(&self) -> NodeType {
        NodeType::File
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    #[allow(clippy::cast_possible_truncation)]
    fn size(&self) -> usize {
        block::capacity() as usize
    }

    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, Errno> {
        block::read(offset, buf)
    }

    fn write(&self, offset: usize, data: &[u8]) -> Result<usize, Errno> {
        block::write(offset, data)
    }

    fn truncate(&self) -> Result<(), Errno> {
        Err(Errno::EINVAL)
    }
}

pub(super) fn new() -> Arc<StaticDir> {
    StaticDir::new(vec![
        ("null", Arc::new(DevNull { ino: alloc_ino() })),
        ("zero", Arc::new(DevZero { ino: alloc_ino() })),
        ("vda", Arc::new(DevVda { ino: alloc_ino() })),
    ])
}
