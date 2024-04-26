use core::net::Ipv4Addr;

use alloc::vec::Vec;
use common::mutex::Mutex;

use crate::{
    debug,
    drivers::virtio::net::NetworkDevice,
    info,
    net::{ipv4::IpV4Header, udp::UdpHeader},
};

use self::{ethernet::EthernetHeader, mac::MacAddress};

mod arp;
mod ethernet;
mod ipv4;
pub mod mac;
mod udp;

static NETWORK_DEVICE: Mutex<Option<NetworkDevice>> = Mutex::new(None);
static IP_ADDR: Ipv4Addr = Ipv4Addr::new(10, 0, 2, 15);

pub fn assign_network_device(device: NetworkDevice) {
    *NETWORK_DEVICE.lock() = Some(device);
}

pub fn receive_and_process_packets() {
    let packets = NETWORK_DEVICE
        .lock()
        .as_mut()
        .expect("There must be a configured network device.")
        .receive_packets();

    for packet in packets {
        process_packet(packet);
    }
}

pub fn send_packet(packet: Vec<u8>) {
    NETWORK_DEVICE
        .lock()
        .as_mut()
        .expect("There must be a configured network device.")
        .send_packet(packet)
        .expect("Packet must be sendable");
}

pub fn current_mac_address() -> MacAddress {
    NETWORK_DEVICE
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
        ethernet::EtherTypes::Arp => {
            arp::process_and_respond(rest);
        }
        ethernet::EtherTypes::IPv4 => {
            let (ipv4_header, rest) =
                IpV4Header::process(rest).expect("IPv4 packet must be processed.");
            // We already asserted that it must be UDP in the IpV4Header::process method
            let (udp_header, data) =
                UdpHeader::process(rest, ipv4_header).expect("Udp header must be valid.");
            let text = core::str::from_utf8(data).expect("Must be valid text.");
            info!("Got data: {text}");
        }
    }
}
