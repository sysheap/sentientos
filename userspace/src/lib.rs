#![no_std]
#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(linkage)]
#![feature(c_size_t)]

mod _start;
mod args;
mod heap;
pub mod net;
mod panic;
pub mod posix;
pub mod print;
pub mod util;

pub use args::{Args, args};
