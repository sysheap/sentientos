use common::big_endian::BigEndian;

use crate::{
    assert::static_assert_size,
    info,
    klibc::util::{BufferExtension, ByteInterpretable},
};

use super::ipv4::IpV4Header;

#[derive(Debug)]
#[repr(C)]
pub struct UdpHeader {
    source_port: BigEndian<u16>,
    destination_port: BigEndian<u16>,
    length: BigEndian<u16>,
    checksum: BigEndian<u16>,
}

static_assert_size!(UdpHeader, 8);

impl ByteInterpretable for UdpHeader {}

#[derive(Debug)]
pub enum UdpParseError {
    PacketTooSmall,
}

impl UdpHeader {
    const UDP_HEADER_SIZE: usize = core::mem::size_of::<Self>();
    const UDP_PROTOCOL_TYPE: u8 = 17;

    pub fn destination_port(&self) -> u16 {
        self.destination_port.get()
    }

    pub fn process<'a>(
        data: &'a [u8],
        ip_header: &IpV4Header,
    ) -> Result<(&'a UdpHeader, &'a [u8]), UdpParseError> {
        if data.len() < Self::UDP_HEADER_SIZE {
            return Err(UdpParseError::PacketTooSmall);
        }

        let (udp_header, rest) = data.split_as::<UdpHeader>();

        info!(
            "Received udp packet; Header tells {:#x} length and we got {:#x} rest of data",
            udp_header.length.get(),
            rest.len()
        );
        assert!(
            rest.len() + Self::UDP_HEADER_SIZE >= udp_header.length.get() as usize,
            "The length field must have a valid value."
        );

        // Truncate data field
        let data_length = udp_header.length.get() as usize - Self::UDP_HEADER_SIZE;
        let rest = &rest[..data_length];

        // Check checksum
        assert!(
            udp_header.checksum.get() != 0,
            "we test impl for checksum not zero"
        );

        info!("Got checksum: {:#x}", udp_header.checksum.get());

        let computed_checksum = Self::compute_checksum(rest, udp_header, ip_header);

        assert_eq!(computed_checksum, 0, "must be zero for a valid packet.");

        Ok((udp_header, rest))
    }

    fn compute_checksum(data: &[u8], udp_header: &UdpHeader, ip_header: &IpV4Header) -> u16 {
        let mut sum = 0u32;

        assert_eq!(
            data.len(),
            udp_header.length.get() as usize - UdpHeader::UDP_HEADER_SIZE
        );

        let ip = ip_header.source_ip.to_bits();
        sum += ip >> 16;
        sum += ip & 0xffff;
        let ip = ip_header.destination_ip.to_bits();
        sum += ip >> 16;
        sum += ip & 0xffff;
        sum += Self::UDP_PROTOCOL_TYPE as u32;
        sum += udp_header.length.get() as u32;

        let mut add_buffer = |data: &[u8]| {
            let mut addr = 0;
            let mut count = data.len();

            while count > 1 {
                sum += ((data[addr] as u16) << 8 | (data[addr + 1] as u16)) as u32;
                if sum & 0x80000000 != 0 {
                    sum = (sum & 0xffff) | (sum >> 16);
                }
                addr += 2;
                count -= 2;
            }

            if count > 0 {
                sum += (data[addr] as u32) << 8;
            }
        };
        add_buffer(udp_header.as_slice());
        add_buffer(data);

        while sum >> 16 != 0 {
            sum = (sum & 0xffff) + (sum >> 16);
        }

        let checksum = !(sum as u16);

        checksum
    }
}

#[cfg(test)]
mod tests {
    use common::big_endian::BigEndian;

    use crate::net::ipv4::IpV4Header;
    use core::net::Ipv4Addr;

    use super::UdpHeader;

    #[test_case]
    fn checksum_calculation() {
        let mut ip_header = IpV4Header {
            version_and_ihl: BigEndian::from_little_endian(0),
            tos: BigEndian::from_little_endian(0),
            total_packet_length: BigEndian::from_little_endian(0),
            identification: BigEndian::from_little_endian(0),
            flags_and_offset: BigEndian::from_little_endian(0),
            ttl: BigEndian::from_little_endian(0),
            upper_protocol: BigEndian::from_little_endian(0),
            header_checksum: BigEndian::from_little_endian(0),
            source_ip: Ipv4Addr::new(10, 0, 2, 2),
            destination_ip: Ipv4Addr::new(10, 0, 2, 15),
        };

        let mut udp_header = UdpHeader {
            source_port: BigEndian::from_little_endian(33015),
            destination_port: BigEndian::from_little_endian(1234),
            length: BigEndian::from_little_endian(21),
            checksum: BigEndian::from_little_endian(0x05fb),
        };

        let data = "Hello World!\n";

        let calculated_checksum =
            UdpHeader::compute_checksum(data.as_bytes(), &udp_header, &ip_header);

        assert_eq!(calculated_checksum, 0);
    }
}
