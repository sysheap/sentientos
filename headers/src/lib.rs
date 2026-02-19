#![no_std]
#![allow(non_camel_case_types)]

pub mod syscalls {
    include!(concat!(env!("OUT_DIR"), "/syscalls.rs"));
}

#[allow(unsafe_op_in_unsafe_fn, clippy::missing_safety_doc)]
pub mod syscall_types {
    include!(concat!(env!("OUT_DIR"), "/syscall_types.rs"));
}

pub mod errno {
    include!(concat!(env!("OUT_DIR"), "/errno.rs"));
}
