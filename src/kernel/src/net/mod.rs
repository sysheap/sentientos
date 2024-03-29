use alloc::vec::Vec;
use common::mutex::Mutex;

use crate::{debug, drivers::virtio::net::NetworkDevice};

use self::{ethernet::EthernetHeader, mac::MacAddress};

mod arp;
mod ethernet;
pub mod mac;

static NETWORK_DEVICE: Mutex<Option<NetworkDevice>> = Mutex::new(None);

pub fn assign_network_device(device: NetworkDevice) {
    *NETWORK_DEVICE.lock() = Some(device);
}

pub fn receive_packets() -> Vec<Vec<u8>> {
    let packets = NETWORK_DEVICE
        .lock()
        .as_mut()
        .expect("There must be a configured network device.")
        .receive_packets();

    let processed_packets = packets
        .into_iter()
        .filter_map(|p| process_packet(p))
        .collect();

    processed_packets
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

    let ether_type = ethernet_header.ether_type();

    match ether_type {
        ethernet::EtherTypes::Arp => {
            debug!("Received ARP packet");
            arp::process_and_respond(rest);
            return None;
        }
    }

    Some(packet)
}
