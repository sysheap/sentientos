use core::arch::asm;

/// The registers regarding syscalls are filled in the following way:
/// a0: syscall number / return value
/// a1: arg1
/// a2: arg2
/// ... and so on

pub fn ecall_0(nr: usize) -> (usize, usize) {
    let ret1: usize;
    let ret2: usize;
    unsafe {
        asm!(
            "ecall",
            in("a0") nr,
            lateout("a0") ret1,
            lateout("a1") ret2,
        );
    }
    (ret1, ret2)
}

pub fn ecall_1(nr: usize, arg1: usize) -> (usize, usize) {
    let ret1: usize;
    let ret2: usize;
    unsafe {
        asm!(
            "ecall",
            in("a0") nr,
            in("a1") arg1,
            lateout("a0") ret1,
            lateout("a1") ret2,
        );
    }
    (ret1, ret2)
}

pub fn ecall_2(nr: usize, arg1: usize, arg2: usize) -> (usize, usize) {
    let ret1: usize;
    let ret2: usize;
    unsafe {
        asm!(
            "ecall",
            in("a0") nr,
            in("a1") arg1,
            in("a2") arg2,
            lateout("a0") ret1,
            lateout("a1") ret2,
        );
    }
    (ret1, ret2)
}

pub fn ecall_3(nr: usize, arg1: usize, arg2: usize, arg3: usize) -> (usize, usize) {
    let ret1: usize;
    let ret2: usize;
    unsafe {
        asm!(
            "ecall",
            in("a0") nr,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            lateout("a0") ret1,
            lateout("a1") ret2,
        );
    }
    (ret1, ret2)
}
