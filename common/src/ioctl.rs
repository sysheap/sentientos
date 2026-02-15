pub const SENTIENT_PANIC: u32 = 0x5301;
pub const SENTIENT_LIST_PROGRAMS: u32 = 0x5302;

const NR_IOCTL: usize = 29;

pub fn trigger_kernel_panic() {
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") 1usize => _,
            in("a1") SENTIENT_PANIC as usize,
            in("a7") NR_IOCTL,
        );
    }
}

pub fn print_programs() {
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") 1usize => _,
            in("a1") SENTIENT_LIST_PROGRAMS as usize,
            in("a7") NR_IOCTL,
        );
    }
}
