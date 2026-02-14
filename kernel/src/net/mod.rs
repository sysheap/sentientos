use core::{cell::LazyCell, net::Ipv4Addr};

use alloc::vec::Vec;

use crate::{
    debug,
    drivers::virtio::net::NetworkDevice,
    klibc::Spinlock,
    net::{ipv4::IpV4Header, udp::UdpHeader},
};

use self::{ethernet::EthernetHeader, mac::MacAddress, sockets::OpenSockets};

pub mod arp;
mod checksum;
mod ethernet;
mod ipv4;
pub mod mac;
pub mod sockets;
pub mod udp;

struct NetworkStack {
    device: Spinlock<Option<NetworkDevice>>,
    ip_addr: Ipv4Addr,
    open_sockets: Spinlock<LazyCell<OpenSockets>>,
}

static NETWORK_STACK: NetworkStack = NetworkStack {
    device: Spinlock::new(None),
    ip_addr: Ipv4Addr::new(10, 0, 2, 15),
    open_sockets: Spinlock::new(LazyCell::new(OpenSockets::new)),
};

pub fn ip_addr() -> Ipv4Addr {
    NETWORK_STACK.ip_addr
}

pub fn open_sockets() -> &'static Spinlock<LazyCell<OpenSockets>> {
    &NETWORK_STACK.open_sockets
}

pub fn assign_network_device(device: NetworkDevice) {
    *NETWORK_STACK.device.lock() = Some(device);
}

pub fn receive_and_process_packets() {
    let packets = NETWORK_STACK
        .device
        .lock()
        .as_mut()
        .expect("There must be a configured network device.")
        .receive_packets();

    for packet in packets {
        process_packet(packet);
    }
}

pub fn send_packet(packet: Vec<u8>) {
    NETWORK_STACK
        .device
        .lock()
        .as_mut()
        .expect("There must be a configured network device.")
        .send_packet(packet)
        .expect("Packet must be sendable");
}

pub fn current_mac_address() -> MacAddress {
    NETWORK_STACK
        .device
        .lock()
        .as_ref()
        .expect("There must be a configured network device.")
        .get_mac_address()
}

fn process_packet(packet: Vec<u8>) {
    let (ethernet_header, rest) = match EthernetHeader::try_parse(&packet) {
        Ok(p) => p,
        Err(err) => {
            debug!("Could not parse ethernet header: {:?}", err);
            return;
        }
    };

    debug!("Received ethernet packet: {}", ethernet_header);

    let ether_type = ethernet_header.ether_type();

    match ether_type {
        ethernet::EtherTypes::Arp => arp::process_and_respond(rest),
        ethernet::EtherTypes::IPv4 => process_ipv4_packet(rest),
    }
}

fn process_ipv4_packet(data: &[u8]) {
    let (ipv4_header, rest) = IpV4Header::process(data).expect("IPv4 packet must be processed.");
    let (udp_header, data) =
        UdpHeader::process(rest, ipv4_header).expect("Udp header must be valid.");
    open_sockets().lock().put_data(
        ipv4_header.source_ip,
        udp_header.source_port(),
        udp_header.destination_port(),
        data,
    );
}
