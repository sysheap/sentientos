use alloc::{string::String, sync::Arc, vec, vec::Vec};
use headers::errno::Errno;

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

pub(super) struct DevDir {
    ino: u64,
    null: VfsNodeRef,
    zero: VfsNodeRef,
}

impl DevDir {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            ino: alloc_ino(),
            null: Arc::new(DevNull { ino: alloc_ino() }),
            zero: Arc::new(DevZero { ino: alloc_ino() }),
        })
    }
}

impl VfsNode for DevDir {
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
        match name {
            "null" => Ok(self.null.clone()),
            "zero" => Ok(self.zero.clone()),
            _ => Err(Errno::ENOENT),
        }
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, Errno> {
        Ok(vec![
            DirEntry {
                name: String::from("null"),
                ino: self.null.ino(),
                node_type: NodeType::File,
            },
            DirEntry {
                name: String::from("zero"),
                ino: self.zero.ino(),
                node_type: NodeType::File,
            },
        ])
    }
}
