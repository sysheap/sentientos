#![no_std]
#![allow(dead_code)]
#![allow(unused_variables)]

mod _start;
mod args;
mod heap;
pub mod net;
mod panic;
pub mod print;
pub mod util;

pub use args::{Args, args};
