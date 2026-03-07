use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use headers::errno::Errno;

use crate::klibc::Spinlock;

use super::vfs::{DirEntry, NodeType, VfsNode, VfsNodeRef, alloc_ino};

pub struct TmpfsFile {
    ino: u64,
    data: Spinlock<Vec<u8>>,
}

impl TmpfsFile {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            ino: alloc_ino(),
            data: Spinlock::new(Vec::new()),
        })
    }
}

impl VfsNode for TmpfsFile {
    fn node_type(&self) -> NodeType {
        NodeType::File
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    fn size(&self) -> usize {
        self.data.lock().len()
    }

    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, Errno> {
        let data = self.data.lock();
        if offset >= data.len() {
            return Ok(0);
        }
        let available = &data[offset..];
        let n = buf.len().min(available.len());
        buf[..n].copy_from_slice(&available[..n]);
        Ok(n)
    }

    fn write(&self, offset: usize, data: &[u8]) -> Result<usize, Errno> {
        let mut content = self.data.lock();
        let end = offset + data.len();
        if end > content.len() {
            content.resize(end, 0);
        }
        content[offset..end].copy_from_slice(data);
        Ok(data.len())
    }

    fn truncate(&self) -> Result<(), Errno> {
        self.data.lock().clear();
        Ok(())
    }
}

pub struct TmpfsDir {
    ino: u64,
    children: Spinlock<BTreeMap<String, VfsNodeRef>>,
}

impl TmpfsDir {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            ino: alloc_ino(),
            children: Spinlock::new(BTreeMap::new()),
        })
    }
}

impl VfsNode for TmpfsDir {
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
        self.children.lock().get(name).cloned().ok_or(Errno::ENOENT)
    }

    fn create(&self, name: &str, node_type: NodeType) -> Result<VfsNodeRef, Errno> {
        let mut children = self.children.lock();
        if children.contains_key(name) {
            return Err(Errno::EEXIST);
        }
        let node: VfsNodeRef = match node_type {
            NodeType::File => TmpfsFile::new(),
            NodeType::Directory => TmpfsDir::new(),
        };
        children.insert(name.to_string(), node.clone());
        Ok(node)
    }

    fn unlink(&self, name: &str) -> Result<(), Errno> {
        self.children
            .lock()
            .remove(name)
            .map(|_| ())
            .ok_or(Errno::ENOENT)
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, Errno> {
        let children = self.children.lock();
        Ok(children
            .iter()
            .map(|(name, node)| DirEntry {
                name: name.clone(),
                ino: node.ino(),
                node_type: node.node_type(),
            })
            .collect())
    }
}
