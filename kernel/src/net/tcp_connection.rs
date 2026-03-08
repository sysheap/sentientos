use crate::{debug, net::mac::MacAddress};

use super::{ipv4::IpV4Header, tcp::TcpHeader};

pub fn process_tcp_packet(ip_header: &IpV4Header, data: &[u8], _source_mac: MacAddress) {
    let (tcp_header, payload) = match TcpHeader::process(data, ip_header) {
        Ok(result) => result,
        Err(e) => {
            debug!("TCP parse error: {:?}", e);
            return;
        }
    };

    debug!(
        "TCP packet: {}:{} -> {}:{} flags={:#x} seq={} ack={} len={}",
        ip_header.source_ip,
        tcp_header.source_port(),
        ip_header.destination_ip,
        tcp_header.destination_port(),
        tcp_header.flags(),
        tcp_header.sequence_number(),
        tcp_header.acknowledgment_number(),
        payload.len(),
    );
}
