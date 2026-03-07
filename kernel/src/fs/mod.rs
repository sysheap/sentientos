pub mod open_file;
mod procfs;
mod tmpfs;
pub mod vfs;

pub use open_file::VfsOpenFile;
pub use vfs::{resolve_parent, resolve_path, resolve_relative};

pub fn init() {
    vfs::mount("/", vfs::RootDir::new());
    vfs::mount("/tmp", tmpfs::TmpfsDir::new());
    vfs::mount("/proc", procfs::ProcDir::new());
}
