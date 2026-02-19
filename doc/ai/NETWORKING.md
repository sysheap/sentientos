# Networking

## Overview

Network stack implementing:
- Ethernet frame parsing
- ARP (Address Resolution Protocol)
- IPv4 packet handling
- UDP sockets

Currently UDP only - no TCP support.

## Architecture

```
                Application (userspace)
                      |
                socket / bind / sendto / recvfrom (Linux syscalls)
                      |
                +-----------+
                |  Sockets  |  kernel/src/net/sockets.rs
                +-----------+
                      |
                +-----+-----+
                |    UDP    |  kernel/src/net/udp.rs
                +-----------+
                      |
                +-----+-----+
                |   IPv4    |  kernel/src/net/ipv4.rs
                +-----------+
                      |
    +--------+  +-----+-----+
    |  ARP   |--|  Ethernet |  kernel/src/net/ethernet.rs
    +--------+  +-----------+
                      |
            +------------------+
            |  VirtIO Network  |  kernel/src/drivers/virtio/net/
            +------------------+
```

## Global State

**File:** `kernel/src/net/mod.rs`

```rust
static NETWORK_DEVICE: Spinlock<Option<NetworkDevice>> = Spinlock::new(None);
static IP_ADDR: Ipv4Addr = Ipv4Addr::new(10, 0, 2, 15);  // QEMU default
pub static ARP_CACHE: Spinlock<BTreeMap<Ipv4Addr, MacAddress>> = Spinlock::new(BTreeMap::new());
pub static OPEN_UDP_SOCKETS: Spinlock<LazyCell<OpenSockets>> = ...;
```

## Packet Reception Flow

```rust
pub fn receive_and_process_packets() {
    let packets = NETWORK_DEVICE.lock().receive_packets();
    for packet in packets {
        process_packet(packet);
    }
}

fn process_packet(packet: Vec<u8>) {
    let (ethernet_header, rest) = EthernetHeader::try_parse(&packet)?;

    match ethernet_header.ether_type() {
        EtherTypes::Arp => arp::process_and_respond(rest),
        EtherTypes::IPv4 => {
            let (ipv4_header, rest) = IpV4Header::process(rest)?;
            let (udp_header, data) = UdpHeader::process(rest, ipv4_header)?;
            OPEN_UDP_SOCKETS.lock().put_data(
                ipv4_header.source_ip,
                udp_header.source_port(),
                udp_header.destination_port(),
                data,
            );
        }
    }
}
```

## Socket Management

**File:** `kernel/src/net/sockets.rs`

### OpenSockets

Manages all UDP sockets by port:

```rust
pub struct OpenSockets {
    sockets: SharedSocketMap,  // BTreeMap<Port, WeakSharedAssignedSocket>
}

impl OpenSockets {
    pub fn try_get_socket(&self, port: Port) -> Option<SharedAssignedSocket>
    pub fn put_data(&self, from: Ipv4Addr, from_port: Port, to_port: Port, data: &[u8])
}
```

### AssignedSocket

Individual socket bound to a port. Uses per-datagram buffering — each received datagram is stored with its sender info (matching Linux `recvfrom` semantics).

```rust
struct Datagram {
    from: Ipv4Addr,
    from_port: Port,
    data: Vec<u8>,
}

pub struct AssignedSocket {
    datagrams: VecDeque<Datagram>,    // Received datagrams with sender info
    port: Port,                        // Bound port
    open_sockets: WeakSharedSocketMap, // Back-reference for cleanup
}

impl AssignedSocket {
    pub fn get_port(&self) -> Port
    pub fn get_datagram(&mut self, out_buffer: &mut [u8]) -> Option<(usize, Ipv4Addr, Port)>
}
```

Socket is automatically removed from OpenSockets when dropped.

## Protocol Layers

### Ethernet

**File:** `kernel/src/net/ethernet.rs`

```rust
pub struct EthernetHeader {
    destination: MacAddress,  // 6 bytes
    source: MacAddress,       // 6 bytes
    ether_type: u16,          // 2 bytes
}

pub enum EtherTypes {
    Arp,   // 0x0806
    IPv4,  // 0x0800
}
```

### ARP

**File:** `kernel/src/net/arp.rs`

Handles ARP requests/responses:
- Responds to ARP requests for our IP
- Caches sender's MAC in ARP_CACHE

### IPv4

**File:** `kernel/src/net/ipv4.rs`

```rust
pub struct IpV4Header {
    // Version, IHL, TOS, Length, ID, Flags, TTL, Protocol
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
}
```

Currently only processes UDP protocol (17).

### UDP

**File:** `kernel/src/net/udp.rs`

```rust
pub struct UdpHeader {
    source_port: u16,
    destination_port: u16,
    length: u16,
    checksum: u16,
}

impl UdpHeader {
    pub fn create_udp_packet(
        dest_ip: Ipv4Addr,
        dest_port: u16,
        dest_mac: MacAddress,
        src_port: u16,
        data: &[u8],
    ) -> Vec<u8>
}
```

### MAC Address

**File:** `kernel/src/net/mac.rs`

```rust
pub struct MacAddress([u8; 6]);
```

## Syscall Interface

Networking uses standard Linux syscalls via `std::net::UdpSocket` in userspace.

### socket(AF_INET, SOCK_DGRAM, 0)
Creates an unbound UDP socket fd. `SOCK_CLOEXEC` flag is masked out (no exec to close-on).

### bind(fd, sockaddr_in, addrlen)
Binds the socket to a port. Acquires a socket from the global socket table.

### sendto(fd, buf, len, flags, dest_addr, addrlen)
Sends a UDP packet. Looks up destination MAC via ARP cache, constructs the full UDP/IP/Ethernet packet, and sends via VirtIO.

### recvfrom(fd, buf, len, flags, src_addr, addrlen)
Calls `receive_and_process_packets()` to poll the NIC, then pops the first datagram from the socket's queue. Returns sender address in `src_addr`. Returns `EAGAIN` if no data and `O_NONBLOCK` is set.

### ioctl(fd, FIONBIO, &value)
Sets/clears `O_NONBLOCK` on a socket fd. Used by `std::net::UdpSocket::set_nonblocking()`.

## Userspace Example

```rust
use std::net::UdpSocket;

let socket = UdpSocket::bind("0.0.0.0:1234").expect("bind");
socket.set_nonblocking(true).expect("nonblocking");

let mut buf = [0; 1024];
match socket.recv_from(&mut buf) {
    Ok((n, src)) => {
        socket.send_to(b"reply", src).expect("send");
    }
    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
    Err(e) => panic!("{e}"),
}
```

## QEMU Network Setup

Default configuration (via qemu_wrapper.sh):
- User-mode networking
- Host IP: 10.0.2.2
- Guest IP: 10.0.2.15
- Host can connect to guest ports via port forwarding

## Key Files

| File | Purpose |
|------|---------|
| kernel/src/net/mod.rs | Network module, packet dispatch |
| kernel/src/net/sockets.rs | Socket management |
| kernel/src/net/ethernet.rs | Ethernet frame parsing |
| kernel/src/net/arp.rs | ARP handling |
| kernel/src/net/ipv4.rs | IPv4 parsing |
| kernel/src/net/udp.rs | UDP handling |
| kernel/src/net/mac.rs | MAC address type |
| kernel/src/drivers/virtio/net/ | VirtIO network device |
