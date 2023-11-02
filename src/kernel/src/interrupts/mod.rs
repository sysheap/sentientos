use core::arch::asm;

use self::trap::TrapFrame;

pub mod plic;
pub mod trap;
mod trap_cause;

static mut KERNEL_TRAP_FRAME: TrapFrame = TrapFrame::zero();

pub fn set_sscratch_to_kernel_trap_frame() {
    unsafe {
        asm!("csrw sscratch, {kernel_trap}", kernel_trap = in(reg)&KERNEL_TRAP_FRAME);
    }
}
