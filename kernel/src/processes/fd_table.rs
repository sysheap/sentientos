use alloc::collections::BTreeMap;
use core::fmt;
use headers::errno::Errno;

use crate::{io::stdin_buf::ReadStdin, net::sockets::SharedAssignedSocket, print};

pub type RawFd = i32;

#[derive(Clone)]
pub enum FileDescriptor {
    Stdin,
    Stdout,
    Stderr,
    UdpSocket(SharedAssignedSocket),
}

impl fmt::Debug for FileDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileDescriptor::Stdin => write!(f, "Stdin"),
            FileDescriptor::Stdout => write!(f, "Stdout"),
            FileDescriptor::Stderr => write!(f, "Stderr"),
            FileDescriptor::UdpSocket(_) => write!(f, "UdpSocket(..)"),
        }
    }
}

impl FileDescriptor {
    pub async fn read(&self, count: usize) -> Result<alloc::vec::Vec<u8>, Errno> {
        match self {
            FileDescriptor::Stdin => Ok(ReadStdin::new(count).await),
            _ => Err(Errno::EBADF),
        }
    }

    pub fn write(&self, data: &[u8]) -> Result<usize, Errno> {
        match self {
            FileDescriptor::Stdout | FileDescriptor::Stderr => {
                let s = alloc::string::String::from_utf8_lossy(data);
                print!("{}", s);
                Ok(data.len())
            }
            _ => Err(Errno::EBADF),
        }
    }
}

pub struct FdTable {
    table: BTreeMap<RawFd, FileDescriptor>,
}

impl FdTable {
    pub fn new() -> Self {
        let mut table = BTreeMap::new();
        table.insert(0, FileDescriptor::Stdin);
        table.insert(1, FileDescriptor::Stdout);
        table.insert(2, FileDescriptor::Stderr);
        FdTable { table }
    }

    pub fn get(&self, fd: RawFd) -> Option<&FileDescriptor> {
        self.table.get(&fd)
    }

    pub fn allocate(&mut self, descriptor: FileDescriptor) -> RawFd {
        let fd = (0..).find(|n| !self.table.contains_key(n)).unwrap();
        self.table.insert(fd, descriptor);
        fd
    }

    pub fn close(&mut self, fd: RawFd) -> Result<FileDescriptor, Errno> {
        self.table.remove(&fd).ok_or(Errno::EBADF)
    }
}
