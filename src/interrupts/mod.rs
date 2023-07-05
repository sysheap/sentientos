use core::arch::asm;

use self::trap::TrapFrame;

pub mod plic;
pub mod trap;

static KERNEL_TRAP_FRAME: TrapFrame = TrapFrame::zero();

pub fn set_mscratch_to_kernel_trap_frame() {
    unsafe {
        asm!("csrw mscratch, {kernel_trap}", kernel_trap = in(reg)&KERNEL_TRAP_FRAME);
    }
}
