use core::ffi::{c_int, c_uint};
use headers::{
    errno::Errno,
    socket::{AF_INET, SOCK_CLOEXEC, SOCK_DGRAM, SOCK_STREAM, sockaddr_in},
};

use crate::{
    klibc::util::ByteInterpretable,
    net::{self, arp, sockets::Port, udp::UdpHeader},
    processes::fd_table::FileDescriptor,
    syscalls::linux_validator::LinuxUserspaceArg,
};

use super::linux::{LinuxSyscallHandler, LinuxSyscalls};

impl LinuxSyscallHandler {
    pub(super) fn do_socket(
        &self,
        domain: c_int,
        typ: c_int,
        _protocol: c_int,
    ) -> Result<isize, Errno> {
        assert!(
            domain == AF_INET,
            "socket: only AF_INET supported (got domain={domain})"
        );
        let masked_type = typ & !SOCK_CLOEXEC;
        let descriptor = match masked_type {
            SOCK_DGRAM => FileDescriptor::UnboundUdpSocket,
            SOCK_STREAM => FileDescriptor::UnboundTcpSocket,
            _ => panic!("socket: unsupported type {typ:#x}"),
        };
        let fd = self
            .current_process
            .with_lock(|p| p.fd_table().allocate(descriptor))?;
        Ok(fd as isize)
    }

    pub(super) fn do_bind(
        &self,
        fd: c_int,
        addr: LinuxUserspaceArg<*const u8>,
        addrlen: c_uint,
    ) -> Result<isize, Errno> {
        assert!(
            addrlen as usize >= core::mem::size_of::<sockaddr_in>(),
            "bind: addrlen too small ({addrlen})"
        );

        let descriptor = self
            .current_process
            .with_lock(|p| p.fd_table().get(fd).map(|e| e.descriptor.clone()))
            .ok_or(Errno::EBADF)?;

        assert!(
            matches!(descriptor, FileDescriptor::UnboundUdpSocket),
            "bind: fd {fd} is not an unbound UDP socket"
        );

        if !net::has_network_device() {
            return Err(Errno::ENETDOWN);
        }

        let sin_arg =
            LinuxUserspaceArg::<*const sockaddr_in>::new(addr.raw_arg(), self.get_process());
        let sin = sin_arg.validate_ptr()?;
        let port = Port::new(u16::from_be(sin.sin_port));

        let socket = net::open_sockets()
            .lock()
            .try_get_socket(port)
            .ok_or(Errno::EADDRINUSE)?;

        self.current_process.with_lock(|p| {
            p.fd_table()
                .replace_descriptor(fd, FileDescriptor::UdpSocket(socket))
        })?;

        Ok(0)
    }

    pub(super) fn do_sendto(
        &self,
        fd: c_int,
        buf: LinuxUserspaceArg<*const u8>,
        len: usize,
        _flags: c_int,
        dest_addr: LinuxUserspaceArg<*const u8>,
        addrlen: c_uint,
    ) -> Result<isize, Errno> {
        assert!(
            addrlen as usize >= core::mem::size_of::<sockaddr_in>(),
            "sendto: addrlen too small ({addrlen})"
        );

        let socket = self
            .current_process
            .with_lock(|p| {
                p.fd_table().get(fd).and_then(|e| match &e.descriptor {
                    FileDescriptor::UdpSocket(s) => Some(s.clone()),
                    _ => None,
                })
            })
            .ok_or(Errno::EBADF)?;

        let data = buf.validate_slice(len)?;
        let sin_arg =
            LinuxUserspaceArg::<*const sockaddr_in>::new(dest_addr.raw_arg(), self.get_process());
        let sin = sin_arg.validate_ptr()?;

        let dest_ip = core::net::Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr));
        let dest_port = u16::from_be(sin.sin_port);

        if !net::has_network_device() {
            return Err(Errno::ENETDOWN);
        }

        let destination_mac = if dest_ip == core::net::Ipv4Addr::BROADCAST {
            net::mac::MacAddress::new([0xff, 0xff, 0xff, 0xff, 0xff, 0xff])
        } else {
            arp::cache_lookup(&dest_ip).expect("sendto: destination MAC must be in ARP cache")
        };

        let source_port = socket.lock().get_port().as_u16();
        let packet =
            UdpHeader::create_udp_packet(dest_ip, dest_port, destination_mac, source_port, &data);
        net::send_packet(packet);

        Ok(len as isize)
    }

    pub(super) async fn do_recvfrom(
        &self,
        fd: c_int,
        buf: LinuxUserspaceArg<*mut u8>,
        len: usize,
        _flags: c_int,
        src_addr: LinuxUserspaceArg<Option<*mut u8>>,
        addrlen: LinuxUserspaceArg<Option<*mut c_uint>>,
    ) -> Result<isize, Errno> {
        let (socket, is_nonblocking) = self
            .current_process
            .with_lock(|p| {
                p.fd_table().get(fd).and_then(|e| match &e.descriptor {
                    FileDescriptor::UdpSocket(s) => Some((s.clone(), e.flags.is_nonblocking())),
                    _ => None,
                })
            })
            .ok_or(Errno::EBADF)?;

        let mut tmp_buf = alloc::vec![0u8; len];

        let result = loop {
            let seen = net::sockets::socket_data_counter();
            if let Some(result) = socket.lock().get_datagram(&mut tmp_buf) {
                break result;
            }
            if is_nonblocking {
                return Err(Errno::EAGAIN);
            }
            net::sockets::SocketDataWait::new(seen).await;
        };

        let (bytes_read, from_ip, from_port) = result;
        buf.write_slice(&tmp_buf[..bytes_read])?;

        if src_addr.arg_nonzero() {
            let sin = sockaddr_in {
                sin_family: u16::try_from(AF_INET).expect("AF_INET fits in u16"),
                sin_port: from_port.as_u16().to_be(),
                sin_addr: headers::socket::in_addr {
                    s_addr: u32::from(from_ip).to_be(),
                },
                sin_zero: [0; 8],
            };
            let src_writer =
                LinuxUserspaceArg::<*mut u8>::new(src_addr.raw_arg(), self.get_process());
            src_writer.write_slice(sin.as_slice())?;
            let addrlen_val = c_uint::try_from(core::mem::size_of::<sockaddr_in>())
                .expect("sockaddr_in size fits in c_uint");
            addrlen.write_if_not_none(addrlen_val)?;
        }

        Ok(bytes_read as isize)
    }
}
