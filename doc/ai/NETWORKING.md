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
                sys_open_udp_socket / sys_read_udp_socket / sys_write_back_udp_socket
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
    sockets: SharedSocketMap,  // BTreeMap<u16, WeakSharedAssignedSocket>
}

impl OpenSockets {
    pub fn try_get_socket(&self, port: u16) -> Option<SharedAssignedSocket>
    pub fn put_data(&self, from: Ipv4Addr, from_port: u16, port: u16, data: &[u8])
}
```

### AssignedSocket

Individual socket bound to a port:

```rust
pub struct AssignedSocket {
    buffer: Vec<u8>,                      // Received data
    port: u16,                            // Bound port
    received_from: Option<Ipv4Addr>,      // Last sender IP
    received_port: Option<u16>,           // Last sender port
    open_sockets: WeakSharedSocketMap,    // Back-reference for cleanup
}

impl AssignedSocket {
    pub fn get_port(&self) -> u16
    pub fn get_data(&mut self, out_buffer: &mut [u8]) -> usize
    pub fn get_from(&self) -> Option<Ipv4Addr>
    pub fn get_received_port(&self) -> Option<u16>
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

### Open Socket
```rust
fn sys_open_udp_socket(&mut self, port: UserspaceArgument<u16>)
    -> Result<UDPDescriptor, SysSocketError>
{
    let socket = OPEN_UDP_SOCKETS.lock().try_get_socket(*port)?;
    Ok(process.put_new_udp_socket(socket))
}
```

### Read from Socket
```rust
fn sys_read_udp_socket(&mut self, descriptor, buffer) -> Result<usize, SysSocketError> {
    receive_and_process_packets();  // Poll for new packets
    let buffer = buffer.validate(self)?;
    descriptor.validate(self)?.with_lock(|mut socket| {
        Ok(socket.get_data(buffer))
    })
}
```

### Write to Socket
```rust
fn sys_write_back_udp_socket(&mut self, descriptor, buffer) -> Result<usize, SysSocketError> {
    let recv_ip = socket.get_from()?;
    let recv_port = socket.get_received_port()?;
    let dest_mac = ARP_CACHE.lock().get(&recv_ip)?;

    let packet = UdpHeader::create_udp_packet(
        recv_ip, recv_port, dest_mac, socket.get_port(), buffer
    );
    net::send_packet(packet);
    Ok(buffer.len())
}
```

## Userspace Example

```rust
// Open UDP socket on port 1234
let socket = sys_open_udp_socket(1234)?;

// Receive data
let mut buf = [0u8; 1024];
let n = sys_read_udp_socket(socket, &mut buf)?;

// Send response back to sender
sys_write_back_udp_socket(socket, b"Hello!")?;
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
