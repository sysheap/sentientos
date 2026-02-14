use core::net::Ipv4Addr;

use crate::{
    assert::static_assert_size,
    klibc::{
        big_endian::BigEndian,
        util::{BufferExtension, ByteInterpretable},
    },
};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct IpV4Header {
    pub version_and_ihl: BigEndian<u8>,
    pub tos: BigEndian<u8>,
    pub total_packet_length: BigEndian<u16>,
    pub identification: BigEndian<u16>,
    pub flags_and_offset: BigEndian<u16>,
    pub ttl: BigEndian<u8>,
    pub upper_protocol: BigEndian<u8>,
    pub header_checksum: BigEndian<u16>,
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
    // options_padding: BigEndian<u32>, This field is optional
}

static_assert_size!(IpV4Header, 20);

impl ByteInterpretable for IpV4Header {}

#[derive(Debug)]
pub enum IpV4ParseError {
    PacketTooSmall,
}

const UDP_PROTOCOL_TYPE_UDP: u8 = 17;

impl IpV4Header {
    pub const HEADER_SIZE: usize = core::mem::size_of::<Self>();

    pub fn new(destination_ip: Ipv4Addr, protocol: u8, payload_size: usize) -> Self {
        Self {
            version_and_ihl: BigEndian::from_little_endian((4 << 4) | 5),
            tos: BigEndian::from_little_endian(0),
            total_packet_length: BigEndian::from_little_endian(
                u16::try_from(Self::HEADER_SIZE + payload_size).expect("Size must not exceed u16"),
            ),
            identification: BigEndian::from_little_endian(0),
            flags_and_offset: BigEndian::from_little_endian(0),
            ttl: BigEndian::from_little_endian(128),
            upper_protocol: BigEndian::from_little_endian(protocol),
            header_checksum: BigEndian::from_little_endian(0),
            source_ip: super::IP_ADDR,
            destination_ip,
        }
    }

    pub fn process(data: &[u8]) -> Result<(&IpV4Header, &[u8]), IpV4ParseError> {
        if data.len() < core::mem::size_of::<IpV4Header>() {
            return Err(IpV4ParseError::PacketTooSmall);
        }

        let (ipv4_header, rest) = data.split_as::<IpV4Header>();

        assert!(ipv4_header.total_packet_length.get() as usize <= data.len());

        assert!(
            ipv4_header.flags_and_offset.get() & 0b100 == 0,
            "We don't support fragmented packets yet."
        );

        assert!(
            ipv4_header.destination_ip == super::IP_ADDR,
            "Destination ip address is not ours."
        );

        assert!(
            ipv4_header.upper_protocol.get() == UDP_PROTOCOL_TYPE_UDP,
            "Only UDP is supported for now"
        );

        assert!(
            ipv4_header.checksum_correct(),
            "Checksum must be zero to be correct"
        );
        Ok((ipv4_header, rest))
    }

    pub fn calculate_checksum(&self) -> u16 {
        super::checksum::ones_complement_checksum(&[self.as_slice()])
    }

    fn checksum_correct(&self) -> bool {
        self.calculate_checksum() == 0
    }
}

#[cfg(test)]
mod tests {
    use core::net::Ipv4Addr;

    use crate::klibc::big_endian::BigEndian;

    use super::IpV4Header;

    #[test_case]
    fn checksum_of_zero_header() {
        let header = IpV4Header {
            version_and_ihl: BigEndian::from_little_endian(0),
            tos: BigEndian::from_little_endian(0),
            total_packet_length: BigEndian::from_little_endian(0),
            identification: BigEndian::from_little_endian(0),
            flags_and_offset: BigEndian::from_little_endian(0),
            ttl: BigEndian::from_little_endian(0),
            upper_protocol: BigEndian::from_little_endian(0),
            header_checksum: BigEndian::from_little_endian(0),
            source_ip: Ipv4Addr::new(0, 0, 0, 0),
            destination_ip: Ipv4Addr::new(0, 0, 0, 0),
        };

        assert_eq!(header.calculate_checksum(), 0xFFFF);
    }

    #[test_case]
    fn checksum_validates_correctly() {
        let mut header = IpV4Header {
            version_and_ihl: BigEndian::from_little_endian((4 << 4) | 5),
            tos: BigEndian::from_little_endian(0),
            total_packet_length: BigEndian::from_little_endian(40),
            identification: BigEndian::from_little_endian(0x1234),
            flags_and_offset: BigEndian::from_little_endian(0),
            ttl: BigEndian::from_little_endian(64),
            upper_protocol: BigEndian::from_little_endian(17),
            header_checksum: BigEndian::from_little_endian(0),
            source_ip: Ipv4Addr::new(192, 168, 1, 100),
            destination_ip: Ipv4Addr::new(192, 168, 1, 1),
        };

        let checksum = header.calculate_checksum();
        header.header_checksum = BigEndian::from_little_endian(checksum);

        assert_eq!(header.calculate_checksum(), 0);
    }

    #[test_case]
    fn checksum_detects_corruption() {
        let mut header = IpV4Header {
            version_and_ihl: BigEndian::from_little_endian((4 << 4) | 5),
            tos: BigEndian::from_little_endian(0),
            total_packet_length: BigEndian::from_little_endian(40),
            identification: BigEndian::from_little_endian(0xABCD),
            flags_and_offset: BigEndian::from_little_endian(0),
            ttl: BigEndian::from_little_endian(128),
            upper_protocol: BigEndian::from_little_endian(17),
            header_checksum: BigEndian::from_little_endian(0),
            source_ip: Ipv4Addr::new(10, 0, 2, 15),
            destination_ip: Ipv4Addr::new(10, 0, 2, 2),
        };

        let checksum = header.calculate_checksum();
        header.header_checksum = BigEndian::from_little_endian(checksum);

        assert_eq!(header.calculate_checksum(), 0);

        header.ttl = BigEndian::from_little_endian(64);

        assert_ne!(header.calculate_checksum(), 0);
    }
}
