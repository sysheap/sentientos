use alloc::{collections::BTreeMap, vec::Vec};
use core::fmt;
use headers::{
    errno::Errno,
    syscall_types::{O_CLOEXEC, O_NONBLOCK},
};

use crate::{
    io::{
        pipe::{ReadPipe, SharedPipeBuffer},
        stdin_buf::ReadStdin,
    },
    net::sockets::SharedAssignedSocket,
    print,
};

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
        Self(raw & ((O_NONBLOCK | O_CLOEXEC) as i32))
    }

    #[allow(dead_code)]
    pub fn is_cloexec(self) -> bool {
        (self.0 & O_CLOEXEC as i32) != 0
    }
}

#[derive(Clone)]
pub enum FileDescriptor {
    Stdin,
    Stdout,
    Stderr,
    UnboundUdpSocket,
    UdpSocket(SharedAssignedSocket),
    PipeRead(SharedPipeBuffer),
    PipeWrite(SharedPipeBuffer),
}

impl fmt::Debug for FileDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileDescriptor::Stdin => write!(f, "Stdin"),
            FileDescriptor::Stdout => write!(f, "Stdout"),
            FileDescriptor::Stderr => write!(f, "Stderr"),
            FileDescriptor::UnboundUdpSocket => write!(f, "UnboundUdpSocket"),
            FileDescriptor::UdpSocket(_) => write!(f, "UdpSocket(..)"),
            FileDescriptor::PipeRead(_) => write!(f, "PipeRead(..)"),
            FileDescriptor::PipeWrite(_) => write!(f, "PipeWrite(..)"),
        }
    }
}

impl FileDescriptor {
    pub async fn read(&self, count: usize) -> Result<alloc::vec::Vec<u8>, Errno> {
        match self {
            FileDescriptor::Stdin => Ok(ReadStdin::new(count).await),
            FileDescriptor::PipeRead(buf) => Ok(ReadPipe::new(buf.clone(), count).await),
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
            FileDescriptor::PipeRead(buf) => buf.lock().try_read(count),
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
            FileDescriptor::PipeWrite(buf) => buf.lock().write(data),
            _ => Err(Errno::EBADF),
        }
    }

    pub fn on_close(&self) {
        match self {
            FileDescriptor::PipeRead(buf) => buf.lock().close_read(),
            FileDescriptor::PipeWrite(buf) => buf.lock().close_write(),
            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
pub struct FdEntry {
    pub descriptor: FileDescriptor,
    pub flags: FdFlags,
}

#[derive(Clone)]
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

    pub fn replace_descriptor(
        &mut self,
        fd: RawFd,
        descriptor: FileDescriptor,
    ) -> Result<(), Errno> {
        let entry = self.table.get_mut(&fd).ok_or(Errno::EBADF)?;
        entry.descriptor = descriptor;
        Ok(())
    }

    pub fn dup_to(&mut self, oldfd: RawFd, newfd: RawFd, flags: i32) -> Result<RawFd, Errno> {
        if oldfd == newfd {
            return Err(Errno::EINVAL);
        }
        let entry = self.table.get(&oldfd).ok_or(Errno::EBADF)?.clone();
        if let Some(old_entry) = self.table.remove(&newfd) {
            old_entry.descriptor.on_close();
        }
        self.table.insert(
            newfd,
            FdEntry {
                descriptor: entry.descriptor,
                flags: FdFlags::from_raw(flags),
            },
        );
        Ok(newfd)
    }

    pub fn close(&mut self, fd: RawFd) -> Result<FdEntry, Errno> {
        let entry = self.table.remove(&fd).ok_or(Errno::EBADF)?;
        entry.descriptor.on_close();
        Ok(entry)
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

    #[allow(dead_code)]
    pub fn close_cloexec_fds(&mut self) {
        let cloexec_fds: Vec<RawFd> = self
            .table
            .iter()
            .filter(|(_, entry)| entry.flags.is_cloexec())
            .map(|(&fd, _)| fd)
            .collect();
        for fd in cloexec_fds {
            if let Some(entry) = self.table.remove(&fd) {
                entry.descriptor.on_close();
            }
        }
    }
}
