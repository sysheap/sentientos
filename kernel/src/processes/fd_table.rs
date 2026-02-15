use alloc::collections::BTreeMap;
use core::fmt;
use headers::{errno::Errno, syscall_types::O_NONBLOCK};

use crate::{io::stdin_buf::ReadStdin, net::sockets::SharedAssignedSocket, print};

pub type RawFd = i32;

#[derive(Clone, Copy, Debug, Default)]
pub struct FdFlags(i32);

impl FdFlags {
    pub fn is_nonblocking(self) -> bool {
        (self.0 & O_NONBLOCK as i32) != 0
    }

    pub fn as_raw(self) -> i32 {
        self.0
    }

    pub fn from_raw(raw: i32) -> Self {
        Self(raw & (O_NONBLOCK as i32))
    }
}

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

    pub fn try_read(&self, count: usize) -> Result<alloc::vec::Vec<u8>, Errno> {
        use crate::io::stdin_buf::STDIN_BUFFER;
        match self {
            FileDescriptor::Stdin => {
                let data = STDIN_BUFFER.lock().get(count);
                if data.is_empty() {
                    Err(Errno::EAGAIN)
                } else {
                    Ok(data)
                }
            }
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

#[derive(Clone, Debug)]
pub struct FdEntry {
    pub descriptor: FileDescriptor,
    pub flags: FdFlags,
}

pub struct FdTable {
    table: BTreeMap<RawFd, FdEntry>,
}

impl FdTable {
    pub fn new() -> Self {
        let mut table = BTreeMap::new();
        let default_flags = FdFlags::default();
        table.insert(
            0,
            FdEntry {
                descriptor: FileDescriptor::Stdin,
                flags: default_flags,
            },
        );
        table.insert(
            1,
            FdEntry {
                descriptor: FileDescriptor::Stdout,
                flags: default_flags,
            },
        );
        table.insert(
            2,
            FdEntry {
                descriptor: FileDescriptor::Stderr,
                flags: default_flags,
            },
        );
        FdTable { table }
    }

    pub fn get(&self, fd: RawFd) -> Option<&FdEntry> {
        self.table.get(&fd)
    }

    pub fn allocate(&mut self, descriptor: FileDescriptor) -> Result<RawFd, Errno> {
        let fd = (0..)
            .find(|n| !self.table.contains_key(n))
            .ok_or(Errno::EMFILE)?;
        self.table.insert(
            fd,
            FdEntry {
                descriptor,
                flags: FdFlags::default(),
            },
        );
        Ok(fd)
    }

    pub fn close(&mut self, fd: RawFd) -> Result<FdEntry, Errno> {
        self.table.remove(&fd).ok_or(Errno::EBADF)
    }

    pub fn get_flags(&self, fd: RawFd) -> Result<FdFlags, Errno> {
        self.table.get(&fd).map(|e| e.flags).ok_or(Errno::EBADF)
    }

    pub fn set_flags(&mut self, fd: RawFd, flags: FdFlags) -> Result<(), Errno> {
        self.table
            .get_mut(&fd)
            .map(|e| e.flags = flags)
            .ok_or(Errno::EBADF)
    }
}
