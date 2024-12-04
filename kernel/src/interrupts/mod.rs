use core::ptr::addr_of;

use common::syscalls::trap_frame::TrapFrame;

use crate::{cpu, debug};

pub mod plic;
pub mod trap;
mod trap_cause;

static mut KERNEL_TRAP_FRAME: TrapFrame = TrapFrame::zero();

pub fn set_sscratch_to_kernel_trap_frame() {
    debug!(
        "Set kernel trap frame ({:p}) to sscratch register",
        addr_of!(KERNEL_TRAP_FRAME)
    );
    cpu::write_sscratch_register(addr_of!(KERNEL_TRAP_FRAME));
}
