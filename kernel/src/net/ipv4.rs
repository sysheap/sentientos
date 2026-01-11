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

    /// Code taken from the RFC at https://www.rfc-editor.org/rfc/rfc1071#section-4
    pub fn calculate_checksum(&self) -> u16 {
        let bytes = self.as_slice();

        // Represents the offset but the name is from the RFC
        let mut addr = 0;
        let mut count = bytes.len();

        let mut sum = 0u32;

        while count > 1 {
            // We still have big endian byte order!
            sum += (bytes[addr + 1] as u16 | ((bytes[addr] as u16) << 8)) as u32;
            addr += 2;
            count -= 2;
        }

        if count > 0 {
            sum += bytes[addr] as u32;
        }

        while sum >> 16 != 0 {
            sum = (sum & 0xffff) + (sum >> 16);
        }

        let checksum = !sum;

        checksum as u16
    }

    fn checksum_correct(&self) -> bool {
        self.calculate_checksum() == 0
    }
}
