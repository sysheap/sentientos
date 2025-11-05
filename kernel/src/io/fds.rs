use core::ffi::c_uint;

use crate::{io::stdin_buf::STDIN_BUFFER, print};
use alloc::{collections::BTreeMap, string::String, sync::Arc};
use headers::{errno::Errno, syscall_types::TIOCGWINSZ};

type Fd = Arc<dyn FileDescriptor + Send + Sync>;

pub struct FDs {
    fds: BTreeMap<i32, Fd>,
}

impl FDs {
    pub fn new_with_std() -> Self {
        Self {
            fds: [
                (0, Arc::new(Stdin) as Fd),
                (1, Arc::new(Stdout) as Fd),
                (2, Arc::new(Stderr) as Fd),
            ]
            .into(),
        }
    }

    pub fn get(&self, fd: i32) -> Result<Fd, Errno> {
        self.fds.get(&fd).cloned().ok_or(Errno::EBADF)
    }

    pub fn close(&mut self, fd: i32) -> Result<(), Errno> {
        let fd = self.fds.remove(&fd).ok_or(Errno::EBADF)?;
        fd.close()
    }
}

pub trait FileDescriptor {
    fn read(&self, buf: &mut [u8]) -> Result<usize, Errno>;
    fn write(&self, buf: &[u8]) -> Result<usize, Errno>;
    fn ioctl(&self, op: c_uint) -> Result<isize, Errno>;
    fn close(&self) -> Result<(), Errno>;
}

struct Stdin;
struct Stdout;
struct Stderr;

impl FileDescriptor for Stdout {
    fn read(&self, _buf: &mut [u8]) -> Result<usize, Errno> {
        Err(Errno::EINVAL)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, Errno> {
        print!("{}", String::from_utf8_lossy(buf));
        Ok(buf.len())
    }

    fn ioctl(&self, op: c_uint) -> Result<isize, Errno> {
        if op == TIOCGWINSZ {
            return Err(Errno::ENOTTY);
        }
        Err(Errno::EINVAL)
    }

    fn close(&self) -> Result<(), Errno> {
        Ok(())
    }
}

impl FileDescriptor for Stderr {
    fn read(&self, _buf: &mut [u8]) -> Result<usize, Errno> {
        Err(Errno::EINVAL)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, Errno> {
        print!("{}", String::from_utf8_lossy(buf));
        Ok(buf.len())
    }

    fn ioctl(&self, op: c_uint) -> Result<isize, Errno> {
        if op == TIOCGWINSZ {
            return Err(Errno::ENOTTY);
        }
        Err(Errno::EINVAL)
    }

    fn close(&self) -> Result<(), Errno> {
        Ok(())
    }
}

impl FileDescriptor for Stdin {
    fn read(&self, buf: &mut [u8]) -> Result<usize, Errno> {
        let stdin = STDIN_BUFFER.lock().get(buf.len());
        buf[..stdin.len()].copy_from_slice(&stdin);
        Ok(stdin.len())
    }

    fn write(&self, _buf: &[u8]) -> Result<usize, Errno> {
        Err(Errno::EINVAL)
    }

    fn ioctl(&self, _op: c_uint) -> Result<isize, Errno> {
        Err(Errno::EINVAL)
    }

    fn close(&self) -> Result<(), Errno> {
        Ok(())
    }
}
