use core::{fmt::Display, net::Ipv4Addr};

use alloc::collections::BTreeMap;
use common::{big_endian::BigEndian, mutex::Mutex};

use crate::{
    assert::static_assert_size,
    debug,
    klibc::util::{BufferExtension, ByteInterpretable},
    net::ethernet::{EtherTypes, EthernetHeader},
};

use super::{current_mac_address, mac::MacAddress, IP_ADDR};

const ARP_REQUEST: u16 = 1;
const ARP_RESPONSE: u16 = 2;

const HARDWARE_ADDRESS_TYPE_ETHERNET: u16 = 1;
const PROTOCOL_ADDRESS_TYPE_IPV4: u16 = 0x0800;

static ARP_CACHE: Mutex<BTreeMap<Ipv4Addr, MacAddress>> = Mutex::new(BTreeMap::new());

#[derive(Debug)]
#[repr(C)]
struct ArpPacket {
    hardware_address_type: BigEndian<u16>,
    protocol_address_type: BigEndian<u16>,
    hardware_address_length: BigEndian<u8>,
    protocol_address_length: BigEndian<u8>,
    operation: BigEndian<u16>, // 1: ARP_request 2:ARP_reply
    source_mac_address: MacAddress,
    source_ip_address: Ipv4Addr,
    destination_mac_address: MacAddress,
    destination_ip_address: Ipv4Addr,
}

static_assert_size!(ArpPacket, 28);

impl ByteInterpretable for ArpPacket {}

impl ArpPacket {
    fn new_reply(destination_mac_address: MacAddress, destination_ip_address: Ipv4Addr) -> Self {
        Self {
            hardware_address_type: BigEndian::from_little_endian(HARDWARE_ADDRESS_TYPE_ETHERNET),
            protocol_address_type: BigEndian::from_little_endian(PROTOCOL_ADDRESS_TYPE_IPV4),
            hardware_address_length: BigEndian::from_little_endian(
                core::mem::size_of::<MacAddress>() as u8,
            ),
            protocol_address_length: BigEndian::from_little_endian(
                core::mem::size_of::<Ipv4Addr>() as u8,
            ),
            operation: BigEndian::from_little_endian(ARP_RESPONSE),
            source_mac_address: current_mac_address(),
            source_ip_address: IP_ADDR,
            destination_mac_address,
            destination_ip_address,
        }
    }
}

pub fn process_and_respond(data: &[u8]) {
    if data.len() < core::mem::size_of::<ArpPacket>() {
        panic!("Received ARP packet is too small");
    }

    let arp_header = data.interpret_as::<ArpPacket>();
    assert!(arp_header.hardware_address_type.get() == HARDWARE_ADDRESS_TYPE_ETHERNET); // Ethernet
    assert!(arp_header.protocol_address_type.get() == PROTOCOL_ADDRESS_TYPE_IPV4); // IPv4
    assert!(
        arp_header.hardware_address_length.get() as usize == core::mem::size_of::<MacAddress>()
    ); // MAC address length
    assert!(arp_header.protocol_address_length.get() as usize == core::mem::size_of::<Ipv4Addr>()); // IPv4 address length
    assert!(arp_header.operation.get() == ARP_REQUEST);
    debug!("Received: {:#}", arp_header);

    if arp_header.destination_ip_address != super::IP_ADDR {
        return;
    }

    ARP_CACHE
        .lock()
        .insert(arp_header.source_ip_address, arp_header.source_mac_address);

    let arp_reply =
        ArpPacket::new_reply(arp_header.source_mac_address, arp_header.source_ip_address);

    let ethernet_reply = EthernetHeader::new(
        arp_header.source_mac_address,
        current_mac_address(),
        EtherTypes::Arp,
    );

    let data = [ethernet_reply.as_slice(), arp_reply.as_slice()].concat();
    debug!(
        "ARP respond\n\tethernet: {}\n\tarp: {}",
        ethernet_reply, arp_reply
    );

    super::send_packet(data);
}

impl Display for ArpPacket {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "ARP packet: source_mac: {}, source_ip: {}, destination_mac: {}, destination_ip: {}",
            &self.source_mac_address,
            &self.source_ip_address,
            &self.destination_mac_address,
            &self.destination_ip_address
        )
    }
}
