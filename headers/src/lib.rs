#![no_std]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

pub mod syscalls {
    include!(concat!(env!("OUT_DIR"), "/syscalls.rs"));
}

pub mod syscall_types {
    include!(concat!(env!("OUT_DIR"), "/syscall_types.rs"));
}

pub mod errno {
    include!(concat!(env!("OUT_DIR"), "/errno.rs"));
}
