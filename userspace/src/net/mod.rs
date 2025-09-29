use common::{
    errors::SysSocketError,
    net::UDPDescriptor,
    syscalls::{sys_open_udp_socket, sys_read_udp_socket, sys_write_back_udp_socket},
};

pub struct UdpSocket(UDPDescriptor);

impl UdpSocket {
    pub fn try_open(port: u16) -> Result<Self, SysSocketError> {
        sys_open_udp_socket(port).map(Self)
    }

    pub fn receive(&mut self, buffer: &mut [u8]) -> usize {
        sys_read_udp_socket(self.0, buffer)
            .expect("This must succeed since it is a valid descriptor.")
    }

    pub fn transmit(&mut self, buffer: &[u8]) -> usize {
        sys_write_back_udp_socket(self.0, buffer).expect("Sending must be successful.")
    }
}
