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
mod heap;
mod init;
mod mmio;
mod page_allocator;
mod page_tables;
mod panic;
mod plic;
mod println;
mod trap;
mod uart;
mod util;

extern crate alloc;
