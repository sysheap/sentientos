#![no_std]
#![feature(macro_metavar_expr_concat)]

#[cfg(target_arch = "riscv64")]
mod riscv64;
#[cfg(target_arch = "riscv64")]
pub use riscv64::*;

#[cfg(not(target_arch = "riscv64"))]
mod stub;
#[cfg(not(target_arch = "riscv64"))]
pub use stub::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuId(usize);

impl CpuId {
    pub fn from_hart_id(hart_id: usize) -> Self {
        Self(hart_id)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }
}

impl core::fmt::Display for CpuId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}
