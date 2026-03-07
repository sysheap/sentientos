#![no_std]
#![allow(non_camel_case_types)]

pub mod syscalls {
    include!(concat!(env!("OUT_DIR"), "/syscalls.rs"));
}

pub mod syscall_types {
    include!(concat!(env!("OUT_DIR"), "/syscall_types.rs"));
}

pub mod errno {
    include!(concat!(env!("OUT_DIR"), "/errno.rs"));
}

pub mod socket {
    include!(concat!(env!("OUT_DIR"), "/socket_types.rs"));
}

pub mod fs {
    pub const AT_FDCWD: i32 = -100;
    pub const AT_REMOVEDIR: i32 = 0x200;
    pub const AT_EMPTY_PATH: i32 = 0x1000;
    pub const SEEK_SET: i32 = 0;
    pub const SEEK_CUR: i32 = 1;
    pub const SEEK_END: i32 = 2;
    pub const DT_DIR: u8 = 4;
    pub const DT_REG: u8 = 8;
    pub const S_IFMT: u32 = 0o170000;
    pub const S_IFREG: u32 = 0o100000;
    pub const S_IFDIR: u32 = 0o040000;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct stat {
        pub st_dev: u64,
        pub st_ino: u64,
        pub st_mode: u32,
        pub st_nlink: u32,
        pub st_uid: u32,
        pub st_gid: u32,
        pub st_rdev: u64,
        pub __pad1: u64,
        pub st_size: i64,
        pub st_blksize: i32,
        pub __pad2: i32,
        pub st_blocks: i64,
        pub st_atime: i64,
        pub st_atime_nsec: i64,
        pub st_mtime: i64,
        pub st_mtime_nsec: i64,
        pub st_ctime: i64,
        pub st_ctime_nsec: i64,
        pub __unused4: i32,
        pub __unused5: i32,
    }

    #[repr(C)]
    pub struct linux_dirent64 {
        pub d_ino: u64,
        pub d_off: i64,
        pub d_reclen: u16,
        pub d_type: u8,
        // d_name follows (flexible array)
    }
}
