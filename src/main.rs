#![no_std]
#![no_main]
#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(panic_info_message)]
#![feature(pointer_byte_offsets)]
#![feature(strict_provenance)]
#![feature(nonzero_ops)]
#![feature(core_intrinsics)]

mod asm;
mod init;
mod interrupts;
mod io;
mod klibc;
mod memory;
mod panic;

extern crate alloc;
