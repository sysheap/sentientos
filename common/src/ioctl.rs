pub const SOLAYA_PANIC: u32 = 0x5301;
pub const SOLAYA_LIST_PROGRAMS: u32 = 0x5302;

pub const SIOCGIFHWADDR: u32 = 0x8927;
pub const SIOCSIFADDR: u32 = 0x8916;
pub const ARPHRD_ETHER: u16 = 1;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Ifreq {
    pub ifr_name: [u8; 16],
    pub ifr_data: [u8; 16],
}

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

#[cfg(target_arch = "riscv64")]
pub fn get_mac_address(socket_fd: i32) -> Option<[u8; 6]> {
    let mut ifreq = Ifreq {
        ifr_name: [0; 16],
        ifr_data: [0; 16],
    };
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") socket_fd as usize => ret,
            in("a1") SIOCGIFHWADDR as usize,
            in("a2") &mut ifreq as *mut Ifreq as usize,
            in("a7") NR_IOCTL,
        );
    }
    if ret != 0 {
        return None;
    }
    // MAC is at ifr_data[2..8] (after sa_family u16)
    let mut mac = [0u8; 6];
    mac.copy_from_slice(&ifreq.ifr_data[2..8]);
    Some(mac)
}

#[cfg(target_arch = "riscv64")]
pub fn set_ip_address(socket_fd: i32, ip: [u8; 4]) {
    let mut ifreq = Ifreq {
        ifr_name: [0; 16],
        ifr_data: [0; 16],
    };
    // sockaddr_in: sa_family=AF_INET(2) as u16 LE, then sin_port(2 bytes), then sin_addr(4 bytes)
    ifreq.ifr_data[0] = 2; // AF_INET low byte
    ifreq.ifr_data[1] = 0; // AF_INET high byte
    // sin_port = 0 (bytes 2-3)
    // sin_addr at bytes 4-7
    ifreq.ifr_data[4..8].copy_from_slice(&ip);
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") socket_fd as usize => ret,
            in("a1") SIOCSIFADDR as usize,
            in("a2") &mut ifreq as *mut Ifreq as usize,
            in("a7") NR_IOCTL,
        );
    }
    assert!(ret == 0, "ioctl SIOCSIFADDR failed");
}
