use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};
use headers::errno::Errno;

use crate::klibc::Spinlock;

static NEXT_INO: AtomicU64 = AtomicU64::new(1);

pub fn alloc_ino() -> u64 {
    NEXT_INO.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    File,
    Directory,
}

#[derive(Clone)]
pub struct DirEntry {
    pub name: String,
    pub ino: u64,
    pub node_type: NodeType,
}

pub type VfsNodeRef = Arc<dyn VfsNode>;

pub trait VfsNode: Send + Sync {
    fn node_type(&self) -> NodeType;
    fn ino(&self) -> u64;
    fn size(&self) -> usize;

    fn read(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize, Errno> {
        Err(Errno::EISDIR)
    }

    fn write(&self, _offset: usize, _data: &[u8]) -> Result<usize, Errno> {
        Err(Errno::EISDIR)
    }

    fn truncate(&self) -> Result<(), Errno> {
        Err(Errno::EISDIR)
    }

    fn lookup(&self, _name: &str) -> Result<VfsNodeRef, Errno> {
        Err(Errno::ENOTDIR)
    }

    fn create(&self, _name: &str, _node_type: NodeType) -> Result<VfsNodeRef, Errno> {
        Err(Errno::ENOTDIR)
    }

    fn unlink(&self, _name: &str) -> Result<(), Errno> {
        Err(Errno::ENOTDIR)
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, Errno> {
        Err(Errno::ENOTDIR)
    }
}

static MOUNT_TABLE: Spinlock<BTreeMap<String, VfsNodeRef>> = Spinlock::new(BTreeMap::new());

pub fn mount(path: &str, root: VfsNodeRef) {
    MOUNT_TABLE.lock().insert(path.to_string(), root);
}

pub fn resolve_path(path: &str) -> Result<VfsNodeRef, Errno> {
    let table = MOUNT_TABLE.lock();
    let (mount_path, node) = find_mount(&table, path)?;
    let remainder = &path[mount_path.len()..];
    drop(table);
    walk(node, remainder)
}

pub fn resolve_parent(path: &str) -> Result<(VfsNodeRef, &str), Errno> {
    let last_slash = path.rfind('/').ok_or(Errno::EINVAL)?;
    let parent_path = if last_slash == 0 {
        "/"
    } else {
        &path[..last_slash]
    };
    let name = &path[last_slash + 1..];
    if name.is_empty() {
        return Err(Errno::EINVAL);
    }
    let parent = resolve_path(parent_path)?;
    Ok((parent, name))
}

fn find_mount<'a>(
    table: &'a BTreeMap<String, VfsNodeRef>,
    path: &str,
) -> Result<(&'a str, VfsNodeRef), Errno> {
    let mut best: Option<(&str, &VfsNodeRef)> = None;
    for (mount_path, node) in table.iter() {
        let matches = path == mount_path
            || (path.starts_with(mount_path)
                && path.as_bytes().get(mount_path.len()) == Some(&b'/'));
        if matches
            && (best.is_none() || mount_path.len() > best.as_ref().map(|b| b.0.len()).unwrap_or(0))
        {
            best = Some((mount_path.as_str(), node));
        }
    }
    let (mp, node) = best.ok_or(Errno::ENOENT)?;
    Ok((mp, node.clone()))
}

pub fn resolve_relative(base: VfsNodeRef, path: &str) -> Result<VfsNodeRef, Errno> {
    walk(base, path)
}

fn walk(mut node: VfsNodeRef, path: &str) -> Result<VfsNodeRef, Errno> {
    for component in path.split('/').filter(|c| !c.is_empty()) {
        node = node.lookup(component)?;
    }
    Ok(node)
}

pub(super) struct RootDir {
    ino: u64,
}

impl RootDir {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { ino: alloc_ino() })
    }
}

impl VfsNode for RootDir {
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
        let path = alloc::format!("/{name}");
        MOUNT_TABLE.lock().get(&path).cloned().ok_or(Errno::ENOENT)
    }

    fn readdir(&self) -> Result<Vec<DirEntry>, Errno> {
        let table = MOUNT_TABLE.lock();
        Ok(table
            .iter()
            .filter(|(path, _)| *path != "/")
            .map(|(path, node)| DirEntry {
                name: String::from(&path[1..]),
                ino: node.ino(),
                node_type: node.node_type(),
            })
            .collect())
    }
}
