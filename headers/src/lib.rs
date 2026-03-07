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

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct statx_timestamp {
        pub tv_sec: i64,
        pub tv_nsec: u32,
        pub __reserved: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct statx {
        pub stx_mask: u32,
        pub stx_blksize: u32,
        pub stx_attributes: u64,
        pub stx_nlink: u32,
        pub stx_uid: u32,
        pub stx_gid: u32,
        pub stx_mode: u16,
        pub __spare0: [u16; 1],
        pub stx_ino: u64,
        pub stx_size: u64,
        pub stx_blocks: u64,
        pub stx_attributes_mask: u64,
        pub stx_atime: statx_timestamp,
        pub stx_btime: statx_timestamp,
        pub stx_ctime: statx_timestamp,
        pub stx_mtime: statx_timestamp,
        pub stx_rdev_major: u32,
        pub stx_rdev_minor: u32,
        pub stx_dev_major: u32,
        pub stx_dev_minor: u32,
        pub stx_mnt_id: u64,
        pub stx_dio_mem_align: u32,
        pub stx_dio_offset_align: u32,
        pub stx_subvol: u64,
        pub stx_atomic_write_unit_min: u32,
        pub stx_atomic_write_unit_max: u32,
        pub stx_atomic_write_segments_max: u32,
        pub __spare1: [u32; 1],
        pub __spare3: [u64; 9],
    }

    impl Default for statx {
        fn default() -> Self {
            // Safety: statx is a plain C struct where all-zeros is valid
            unsafe { core::mem::zeroed() }
        }
    }
}
