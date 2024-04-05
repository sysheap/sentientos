use alloc::vec::Vec;
use common::mutex::Mutex;

use crate::{debug, drivers::virtio::net::NetworkDevice};

use self::{ethernet::EthernetHeader, ip_address::IpV4Address, mac::MacAddress};

mod arp;
mod ethernet;
pub mod ip_address;
pub mod mac;

static NETWORK_DEVICE: Mutex<Option<NetworkDevice>> = Mutex::new(None);
static IP_ADDR: IpV4Address = IpV4Address::new(10, 0, 2, 15);

pub fn assign_network_device(device: NetworkDevice) {
    *NETWORK_DEVICE.lock() = Some(device);
}

pub fn receive_packets() -> Vec<Vec<u8>> {
    let packets = NETWORK_DEVICE
        .lock()
        .as_mut()
        .expect("There must be a configured network device.")
        .receive_packets();

    packets.into_iter().filter_map(process_packet).collect()
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

fn process_packet(packet: Vec<u8>) -> Option<Vec<u8>> {
    let (ethernet_header, rest) = match EthernetHeader::try_parse(&packet) {
        Ok(p) => p,
        Err(err) => {
            debug!("Could not parse ethernet header: {:?}", err);
            return None;
        }
    };

    debug!("Received ethernet packet: {}", ethernet_header);

    let ether_type = ethernet_header.ether_type();

    match ether_type {
        ethernet::EtherTypes::Arp => {
            arp::process_and_respond(rest);
            return None;
        }
    }

    Some(packet)
}
