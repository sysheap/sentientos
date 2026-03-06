pub const SOLAYA_PANIC: u32 = 0x5301;
pub const SOLAYA_LIST_PROGRAMS: u32 = 0x5302;

#[cfg(target_arch = "riscv64")]
const NR_IOCTL: usize = 29;

#[cfg(target_arch = "riscv64")]
pub fn trigger_kernel_panic() {
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") 1usize => _,
            in("a1") SOLAYA_PANIC as usize,
            in("a7") NR_IOCTL,
        );
    }
}

#[cfg(target_arch = "riscv64")]
pub fn print_programs() {
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") 1usize => _,
            in("a1") SOLAYA_LIST_PROGRAMS as usize,
            in("a7") NR_IOCTL,
        );
    }
}
