# Networking

## Overview

Network stack implementing:
- Ethernet frame parsing
- ARP (Address Resolution Protocol)
- IPv4 packet handling
- UDP sockets
- TCP connections (client and server)
- DHCP client (dynamic IP configuration)

## Architecture

```
                Application (userspace)
                      |
                socket / bind / sendto / recvfrom / connect / listen / accept
                      |
          +-----------+-----------+
          |                       |
    +-----------+     +-------------------+
    |  Sockets  |     | TCP Connections   |  kernel/src/net/tcp_connection.rs
    | (UDP)     |     | (per-connection   |
    +-----------+     |  async tasks)     |
          |           +-------------------+
    +-----+-----+          |
    |    UDP    |     +-----+-----+
    +-----------+     |    TCP    |  kernel/src/net/tcp.rs
          |           +-----------+
          +-------+-------+
                  |
            +-----+-----+
            |   IPv4    |  kernel/src/net/ipv4.rs
            +-----------+
                  |
    +--------+  +-+---------+
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
static NETWORK_STACK: NetworkStack = NetworkStack {
    device: Spinlock::new(None),
    ip_addr: Spinlock::new(Ipv4Addr::new(0, 0, 0, 0)),
    open_sockets: Spinlock::new(LazyCell::new(OpenSockets::new)),
};
```

**File:** `kernel/src/net/tcp_connection.rs`

```rust
static TCP_CONNECTIONS: Spinlock<BTreeMap<ConnectionId, SharedTcpConnection>> = ...;
static TCP_LISTENERS: Spinlock<BTreeMap<u16, SharedTcpListener>> = ...;
```

IP address starts as `0.0.0.0` and is configured dynamically by the DHCP client at boot.

## Packet Reception Flow

Packet reception is interrupt-driven. `network_rx_task()` is a kernel async task that awaits network interrupts, then processes all available packets.

```rust
pub async fn network_rx_task() {
    loop {
        let seen = NETWORK_INTERRUPT_COUNTER.load(Ordering::SeqCst);
        let count = receive_and_process_packets();
        if count > 0 { sockets::wake_socket_waiters(); }
        NetworkInterruptWait::new(seen).await;
    }
}
```

IPv4 packets are dispatched by protocol:
- Protocol 17 (UDP) → `OpenSockets::put_data()`
- Protocol 6 (TCP) → `tcp_connection::process_tcp_packet()` routes to per-connection mailbox

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

## TCP

**Files:** `kernel/src/net/tcp.rs` (header parsing/creation), `kernel/src/net/tcp_connection.rs` (connection state machine)

### Design: Async State Machine per Connection

Each TCP connection runs as its own async task spawned via `kernel_tasks::spawn()`. The state machine is implicit in the async function's control flow rather than an explicit state enum.

- `network_rx_task()` receives TCP segments and routes them to the correct connection's **segment mailbox** (`VecDeque<ReceivedSegment>` + `Waker`)
- Each connection's async task awaits segments from its mailbox with timeouts
- Retransmission is a natural timeout-retry loop
- State = where the async function is currently suspended

### Simplifications (Minimal TCP)

- No TCP options (ignored incoming, never sent)
- Fixed window size (8192 bytes)
- Fixed MSS (1460, never negotiated)
- No congestion control
- Fixed retransmit timeout (1 second, up to 5 retries)
- Out-of-order packets dropped
- Skips TIME-WAIT state

### Connection Lifecycle

**Server (passive open):**
1. `process_tcp_packet()` receives SYN for a listening port
2. Spawns `server_connection_task()` as async task
3. Sends SYN-ACK, waits for ACK (with retransmission loop)
4. Pushes to listener backlog, enters `established_loop()`

**Client (active open):**
1. `initiate_connect()` sends SYN, awaits SYN-ACK (with retransmission)
2. Sends ACK, spawns `established_loop()` as async task, returns connection

**Established loop:** Awaits segments or user events. Handles incoming data (buffers + ACK), FIN (ACK + close), user close (FIN + wait for ACK), user write (drain send_buffer + send).

### Spinlock Discipline

**Critical rule:** Never call `waker.wake()` or `send_packet()` while holding a `Spinlock`. The kernel's Spinlock is non-reentrant. All lock-then-wake patterns extract data + waker under the lock, then drop the lock before waking/sending.

### Key Types

```rust
pub type SharedTcpConnection = Arc<Spinlock<TcpConnection>>;
pub type SharedTcpListener = Arc<Spinlock<TcpListener>>;
```

### Syscall Integration

musl libc uses `recvfrom`/`sendto` for TCP read/write (not `read`/`write`). Both `do_recvfrom` and `do_sendto` in `net_ops.rs` handle `TcpStream` file descriptors alongside UDP sockets.

`FileDescriptor::read()`/`write()` also support `TcpStream` for direct fd-based I/O.

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

Dispatches to UDP (protocol 17) and TCP (protocol 6). Accepts packets destined for our IP, broadcast (`255.255.255.255`), or any IP when our address is `0.0.0.0` (pre-DHCP).

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

Networking uses standard Linux syscalls via `std::net::UdpSocket` and `std::net::TcpListener`/`TcpStream` in userspace.

### UDP Syscalls

#### socket(AF_INET, SOCK_DGRAM, 0)
Creates an unbound UDP socket fd.

#### bind(fd, sockaddr_in, addrlen)
Binds the socket to a port. Acquires a socket from the global socket table.

#### sendto(fd, buf, len, flags, dest_addr, addrlen)
Sends a UDP packet. Also handles TCP: for `TcpStream` fds, queues data to the connection's send buffer.

#### recvfrom(fd, buf, len, flags, src_addr, addrlen)
For UDP, pops the first datagram from the socket's queue. For TCP, awaits data from the connection's receive buffer. Returns `EAGAIN` if no data and `O_NONBLOCK` is set.

### TCP Syscalls

#### socket(AF_INET, SOCK_STREAM, 0)
Creates an unbound TCP socket fd (`FileDescriptor::UnboundTcpSocket`).

#### bind(fd, sockaddr_in, addrlen)
For TCP, creates a `TcpListener` and replaces the fd descriptor.

#### listen(fd, backlog)
Registers the TCP listener in the global `TCP_LISTENERS` table, making it accept incoming SYN packets.

#### accept(fd, addr, addrlen) / accept4(fd, addr, addrlen, flags)
Awaits a new connection on the listener's backlog. Returns a new fd with `FileDescriptor::TcpStream`.

#### connect(fd, addr, addrlen)
Initiates a TCP three-way handshake (async). On success, replaces the fd with `TcpStream`.

#### setsockopt(fd, level, optname, optval, optlen)
Stubbed to return 0. musl calls `SO_REUSEADDR` before bind.

#### shutdown(fd, how)
Requests connection close. Wakes the connection task to send FIN.

#### getsockname(fd, addr, addrlen)
Returns the local address (IP + port) of the socket.

### Network ioctls

#### ioctl(fd, FIONBIO, &value)
Sets/clears `O_NONBLOCK` on a socket fd.

#### ioctl(fd, SIOCGIFHWADDR, &ifreq)
Returns the NIC's MAC address.

#### ioctl(fd, SIOCSIFADDR, &ifreq)
Sets the kernel's IP address. Used by the DHCP client.

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

**File:** `userspace/src/bin/dhcpd.rs`

The DHCP client runs as a userspace program during boot (spawned by `init` before the shell). It gets the NIC MAC via `ioctl(SIOCGIFHWADDR)`, performs the standard 4-step DHCP handshake (DISCOVER, OFFER, REQUEST, ACK), then configures the kernel IP via `ioctl(SIOCSIFADDR)`.

Prints `dhcpd: configured ip X.X.X.X` on success. Exits cleanly with status 1 if no network device is present (boot continues regardless).

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
| kernel/src/net/mod.rs | Network module, packet dispatch, interrupt handling |
| kernel/src/net/sockets.rs | UDP socket management |
| kernel/src/net/tcp.rs | TCP header parsing, checksum, packet creation |
| kernel/src/net/tcp_connection.rs | TCP connection state machine, listener, async tasks |
| kernel/src/net/ethernet.rs | Ethernet frame parsing |
| kernel/src/net/arp.rs | ARP handling |
| kernel/src/net/ipv4.rs | IPv4 parsing |
| kernel/src/net/udp.rs | UDP handling |
| kernel/src/net/mac.rs | MAC address type |
| kernel/src/drivers/virtio/net/ | VirtIO network device |
| userspace/src/bin/dhcpd.rs | DHCP client |
| userspace/src/bin/tcp_echo.rs | TCP echo server (test program) |
| common/src/ioctl.rs | Network ioctl wrappers (MAC, IP) |
