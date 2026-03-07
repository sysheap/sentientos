use alloc::{string::String, sync::Arc, vec, vec::Vec};
use headers::errno::Errno;

use super::vfs::{DirEntry, NodeType, VfsNode, VfsNodeRef, alloc_ino};

struct ProcVersionFile {
    ino: u64,
}

const VERSION_STRING: &[u8] = b"Solaya 0.1.0\n";

impl VfsNode for ProcVersionFile {
    fn node_type(&self) -> NodeType {
        NodeType::File
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    fn size(&self) -> usize {
        VERSION_STRING.len()
    }

    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, Errno> {
        if offset >= VERSION_STRING.len() {
            return Ok(0);
        }
        let available = &VERSION_STRING[offset..];
        let n = buf.len().min(available.len());
        buf[..n].copy_from_slice(&available[..n]);
        Ok(n)
    }

    fn write(&self, _offset: usize, _data: &[u8]) -> Result<usize, Errno> {
        Err(Errno::EACCES)
    }
}

pub struct ProcDir {
    ino: u64,
    version: VfsNodeRef,
}

impl ProcDir {
    pub fn new() -> Arc<Self> {
        let version: VfsNodeRef = Arc::new(ProcVersionFile { ino: alloc_ino() });
        Arc::new(Self {
            ino: alloc_ino(),
            version,
        })
    }
}

impl VfsNode for ProcDir {
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
            "version" => Ok(self.version.clone()),
            _ => Err(Errno::ENOENT),
        }
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, Errno> {
        Ok(vec![DirEntry {
            name: String::from("version"),
            ino: self.version.ino(),
            node_type: NodeType::File,
        }])
    }
}
