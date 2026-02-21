#![no_std]
#![allow(non_camel_case_types)]

pub mod syscalls {
    include!(concat!(env!("OUT_DIR"), "/syscalls.rs"));
}

pub mod syscall_types {
    include!(concat!(env!("OUT_DIR"), "/syscall_types.rs"));
}

pub mod errno {
    include!(concat!(env!("OUT_DIR"), "/errno.rs"));
}

pub mod socket {
    pub const AF_INET: i32 = 2;
    pub const SOCK_DGRAM: i32 = 2;
    pub const SOCK_CLOEXEC: i32 = 0x80000;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct in_addr {
        pub s_addr: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct sockaddr_in {
        pub sin_family: u16,
        pub sin_port: u16,
        pub sin_addr: in_addr,
        pub sin_zero: [u8; 8],
    }

    impl sockaddr_in {
        pub fn from_bytes(bytes: &[u8]) -> Self {
            assert!(bytes.len() >= core::mem::size_of::<Self>());
            Self {
                sin_family: u16::from_ne_bytes([bytes[0], bytes[1]]),
                sin_port: u16::from_ne_bytes([bytes[2], bytes[3]]),
                sin_addr: in_addr {
                    s_addr: u32::from_ne_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                },
                sin_zero: [
                    bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                    bytes[15],
                ],
            }
        }

        pub fn as_bytes(&self) -> [u8; core::mem::size_of::<Self>()] {
            let mut buf = [0u8; core::mem::size_of::<Self>()];
            buf[0..2].copy_from_slice(&self.sin_family.to_ne_bytes());
            buf[2..4].copy_from_slice(&self.sin_port.to_ne_bytes());
            buf[4..8].copy_from_slice(&self.sin_addr.s_addr.to_ne_bytes());
            buf[8..16].copy_from_slice(&self.sin_zero);
            buf
        }
    }
}
