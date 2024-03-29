use common::big_endian::BigEndian;

use crate::{debug, klibc::util::BufferExtension};

use super::{current_mac_address, mac::MacAddress};

const BROADCAST_MAC: MacAddress = MacAddress::new([0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);

#[repr(packed)]
pub struct EthernetHeader {
    destination_mac: MacAddress,
    source_mac: MacAddress,
    ether_type: BigEndian<u16>,
    // data: [u8],
    // chksum: u32,
}

#[derive(Debug)]
pub enum ParseError {
    PacketTooSmall,
    UnknownEtherType,
    UnknownDestinationMac,
}

const ETHERTYPE_ARP: u16 = 0x0806;

pub enum EtherTypes {
    Arp,
}

impl TryFrom<BigEndian<u16>> for EtherTypes {
    type Error = ParseError;

    fn try_from(value: BigEndian<u16>) -> Result<Self, Self::Error> {
        match value.get() {
            ETHERTYPE_ARP => Ok(EtherTypes::Arp),
            _ => Err(ParseError::UnknownEtherType),
        }
    }
}

impl EthernetHeader {
    const CHECKSUM_LENGTH: usize = core::mem::size_of::<u32>();
    const MIN_LENGTH: usize = core::mem::size_of::<EthernetHeader>() + Self::CHECKSUM_LENGTH; // 4 byte checksum at the end
    pub fn try_parse(data: &[u8]) -> Result<(&Self, &[u8]), ParseError> {
        if data.len() < Self::MIN_LENGTH {
            return Err(ParseError::PacketTooSmall);
        }
        let (header, rest) = data.split_as::<EthernetHeader>();

        if !header.is_valid_ether_type() {
            return Err(ParseError::UnknownEtherType);
        }

        if header.destination_mac != current_mac_address()
            && header.destination_mac != BROADCAST_MAC
        {
            debug!(
                "Unknown destination mac: {}; NIC mac: {}",
                header.destination_mac,
                current_mac_address()
            );
            return Err(ParseError::UnknownDestinationMac);
        }

        Ok((header, rest))
    }

    fn is_valid_ether_type(&self) -> bool {
        EtherTypes::try_from(self.ether_type).is_ok()
    }

    pub fn ether_type(&self) -> EtherTypes {
        EtherTypes::try_from(self.ether_type).expect("Must be already parsed.")
    }
}
