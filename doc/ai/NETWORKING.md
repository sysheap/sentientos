# Networking

## Overview

Network stack implementing:
- Ethernet frame parsing
- ARP (Address Resolution Protocol)
- IPv4 packet handling
- UDP sockets
- DHCP client (dynamic IP configuration)

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
static IP_ADDR: Spinlock<Ipv4Addr> = Spinlock::new(Ipv4Addr::new(0, 0, 0, 0)); // Set by DHCP
pub static ARP_CACHE: Spinlock<BTreeMap<Ipv4Addr, MacAddress>> = Spinlock::new(BTreeMap::new());
pub static OPEN_UDP_SOCKETS: Spinlock<LazyCell<OpenSockets>> = ...;
```

IP address starts as `0.0.0.0` and is configured dynamically by the DHCP client at boot.

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
            // Opportunistically cache source MAC from Ethernet header
            arp::cache_insert(ipv4_header.source_ip, ethernet_header.source_mac());
            let (udp_header, data) = UdpHeader::process(rest, ipv4_header)?;
            OPEN_UDP_SOCKETS.lock().put_data(...);
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
- MAC addresses are also learned opportunistically from incoming IPv4 packets' Ethernet headers via `cache_insert()`

### IPv4

**File:** `kernel/src/net/ipv4.rs`

```rust
pub struct IpV4Header {
    // Version, IHL, TOS, Length, ID, Flags, TTL, Protocol
    pub source_ip: Ipv4Addr,
    pub destination_ip: Ipv4Addr,
}
```

Currently only processes UDP protocol (17). Accepts packets destined for our IP, broadcast (`255.255.255.255`), or any IP when our address is `0.0.0.0` (pre-DHCP).

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
Sends a UDP packet. Returns `ENETDOWN` if no network device. For broadcast destination (`255.255.255.255`), uses broadcast MAC `ff:ff:ff:ff:ff:ff`. Otherwise looks up destination MAC via ARP cache. Constructs the full UDP/IP/Ethernet packet and sends via VirtIO.

### recvfrom(fd, buf, len, flags, src_addr, addrlen)
Calls `receive_and_process_packets()` to poll the NIC, then pops the first datagram from the socket's queue. Returns sender address in `src_addr`. Returns `EAGAIN` if no data and `O_NONBLOCK` is set.

### ioctl(fd, FIONBIO, &value)
Sets/clears `O_NONBLOCK` on a socket fd. Used by `std::net::UdpSocket::set_nonblocking()`.

### ioctl(fd, SIOCGIFHWADDR, &ifreq)
Returns the NIC's MAC address in `ifreq.ifr_data` as `sockaddr` with `sa_family = ARPHRD_ETHER(1)` and MAC in `sa_data[0..6]`. Returns `ENODEV` if no network device.

### ioctl(fd, SIOCSIFADDR, &ifreq)
Sets the kernel's IP address from `sockaddr_in` in `ifreq.ifr_data`. Used by the DHCP client after receiving an address.

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

## DHCP

**File:** `userspace/src/bin/dhcp.rs`

The DHCP client runs as a userspace program during boot (spawned by `init` before the shell). It gets the NIC MAC via `ioctl(SIOCGIFHWADDR)`, performs the standard 4-step DHCP handshake (DISCOVER, OFFER, REQUEST, ACK), then configures the kernel IP via `ioctl(SIOCSIFADDR)`.

Prints `dhcp: configured ip X.X.X.X` on success. Exits cleanly with status 1 if no network device is present (boot continues regardless).

With QEMU user-mode networking, the built-in DHCP server assigns `10.0.2.15`.

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
| userspace/src/bin/dhcp.rs | DHCP client |
| common/src/ioctl.rs | Network ioctl wrappers (MAC, IP) |
