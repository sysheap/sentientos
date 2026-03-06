# Solaya OS - Future Implementation Todos

This document contains research summaries for planned future enhancements. Each item includes implementation complexity, requirements, and key design decisions.

## Table of Contents

1. [Proper Process IDs and Groups](#1-proper-process-ids-and-groups)
2. [Linux-like Signals](#2-linux-like-signals)
3. [Virtual File System (VFS)](#3-virtual-file-system-vfs)
4. [QEMU Block Device Driver](#4-qemu-block-device-driver)
5. [ext2 Filesystem](#5-ext2-filesystem)
6. [Feasible Coreutils](#6-feasible-coreutils)
7. [QEMU Framebuffer](#7-qemu-framebuffer)
8. [Port Doom](#8-port-doom)
9. [Async Network Reception with Interrupts](#9-async-network-reception-with-interrupts)
10. [DHCP Client](#10-dhcp-client)
11. [Minimal TCP Implementation](#11-minimal-tcp-implementation)
12. [Dynamic Linking](#12-dynamic-linking)
13. [QEMU Random Device Driver](#13-qemu-random-device-driver)

---

## 1. Proper Process IDs and Groups

**Complexity:** Medium

### Currently Implemented ✅
- Thread ID (TID): Unique per thread
- Process ID (PID): Returns main thread's TID (partial implementation)
- Parent Process ID (PPID): Tracked via `parent_tid`

### Missing ❌
- **Thread Group ID (TGID)**: Not tracked separately (conflated with PID)
- **Process Group ID (PGID)**: Not tracked at all
- **Session ID (SID)**: Not tracked at all

### Required Changes

**Data Structures:**
Add to `Process` or `Thread`:
```rust
pgid: Tid  // Process group ID
sid: Tid   // Session ID
```

**Syscall Implementations:**
- `getpgid(pid)` - Get PGID (syscall #155)
- `setpgid(pid, pgid)` - Set PGID (syscall #154)
- `getsid(pid)` - Get SID (syscall #156)
- `setsid()` - Create new session (syscall #157)

**Initialization:**
- Init process: `pgid=1`, `sid=1`
- Fork/clone: Child inherits parent's PGID and SID
- Constraints: `setsid()` fails if caller is process group leader

### Purpose
- PGID: Job control (shell pipelines, process groups)
- SID: Session management (terminal control)

**Estimated effort:** 1-2 weeks

---

## 2. Linux-like Signals

**Complexity:** Medium

### Current Signal Support ✅
- `SignalState` struct per thread (sigaltstack, sigmask, sigaction)
- Syscalls: `rt_sigaction`, `rt_sigprocmask`, `sigaltstack`
- Ctrl+C handling (kills process directly, not via signal)

### Missing ❌

**1. Pending Signals Queue**
- No tracking of pending signals (need bitset/queue)
- No per-thread and per-process pending sets

**2. Signal Delivery Mechanism**
- No check for pending signals before returning to userspace
- No handler invocation (jumping to sa_handler)
- No signal frame construction on user stack
- No sa_restorer trampoline for returning

**3. Signal Generation Syscalls**
- `kill(pid, sig)` - Send to process
- `tkill(tid, sig)` - Send to thread
- `tgkill(tgid, tid, sig)` - Send to thread in thread group

**4. Special Semantics**
- SIGCHLD generation on child exit
- SIGKILL/SIGSTOP enforcement (cannot be caught)
- Signal inheritance across fork/clone/execve

### Implementation Approach

**Files to Modify:**
- `kernel/src/processes/thread.rs` - Add `pending_signals` bitset
- `kernel/src/processes/process_table.rs` - Queue signals instead of direct kill
- `kernel/src/interrupts/trap.rs` - Check signals before userspace return
- `kernel/src/syscalls/linux.rs` - Add kill/tkill/tgkill
- `kernel/src/processes/scheduler.rs` - Queue SIGINT instead of direct kill
- New: `kernel/src/processes/signal.rs` - Signal delivery logic

**Key Design Decisions:**
- Standard signals (1-31): bitset (not queued)
- Real-time signals (32-64): queue with siginfo_t
- Check pending signals at: syscall return, interrupt return, schedule points
- Modify user stack to insert signal frame, set PC to handler

**Estimated effort:** 2-3 weeks

---

## 3. Virtual File System (VFS)

**Complexity:** Medium to High

### Core Abstractions

**Traits:**
```rust
trait FileSystem {
    fn root_inode(&self) -> Result<Arc<dyn Inode>>;
    fn superblock(&self) -> &Superblock;
}

trait Inode {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize>;
    fn write(&self, offset: usize, buf: &[u8]) -> Result<usize>;
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>>;
    fn metadata(&self) -> InodeMetadata;
    fn inode_type(&self) -> InodeType;  // File, Dir, Symlink, etc.
}
```

### Modular Filesystem Support
- Global filesystem type registry
- Mount table tracks all mounted filesystems
- Each filesystem type registers with specific operations

### Basic Proc Filesystem

Start with minimal procfs:
```rust
struct ProcFs {
    superblock: Superblock,
}

// Single file: /proc/version
struct ProcVersionFile;

impl Inode for ProcVersionFile {
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        let content = b"Solaya 0.1.0\n";
        // Handle offset and copy
    }
}
```

### Integration with Syscalls

**Extend FileDescriptor enum:**
```rust
pub enum FileDescriptor {
    VfsFile(VfsFile),  // NEW
    Stdin,
    Stdout,
    // ... existing variants
}
```

**Add Syscalls:**
- `openat` - Open files
- `fstat` - File metadata
- `lseek` - Seek position
- `getdents64` - Read directory entries

### Implementation Order
1. Core VFS abstractions (traits, mount table, path resolution)
2. Minimal procfs with `/proc/version`
3. Syscall integration (`openat`, file read/write)
4. Expand procfs (meminfo, cpuinfo, /proc/<pid>/)
5. Additional filesystems (tmpfs, devfs)

**Estimated effort:** 3-4 weeks for core + procfs; ongoing for additional filesystems

---

## 4. QEMU Block Device Driver

**Complexity:** Medium

### Device Specification
- VirtIO Subsystem ID: **2** (vs network = 1)
- Single virtqueue (simpler than network's 2 queues)
- Request structure: 3-descriptor chain (header → data → status)

**Request Format:**
```rust
struct virtio_blk_req {
    type: u32,      // 0=read, 1=write
    reserved: u32,
    sector: u64,    // 512-byte sector offset
    data: [u8],     // Data buffer
    status: u8,     // Device writes: 0=OK, 1=IOERR
}
```

### QEMU Setup
```bash
-drive if=none,file=disk.img,format=raw,id=hd0 \
-device virtio-blk-device,drive=hd0
```

### Implementation Strategy
- **80% code reuse** from existing VirtIO net driver
- Main difference: 3-descriptor chains vs simple buffers
- Sector addressing (512-byte units)
- No need for multiple queues

**Files:**
- `kernel/src/drivers/virtio/block/mod.rs` - New driver
- Reuse: `virtqueue.rs`, `capability.rs`

### DMA and Memory
- Current VirtQueue approach works (direct physical addresses)
- QEMU identity mapping: CPU addresses = DMA addresses
- Memory barriers already handled

**Estimated effort:** 1-2 weeks

---

## 5. ext2 Filesystem

**Complexity:** Medium to High

### On-Disk Format

**Superblock** (at byte 1024):
- Magic: 0xef53
- Block size, inode count, free counts
- Blocks/inodes per group

**Block Groups:**
- Superblock copy, group descriptor table
- Block bitmap, inode bitmap
- Inode table, data blocks

**Inodes** (128 bytes):
- Type/permissions, uid/gid, size, timestamps
- 12 direct + 1 indirect + 1 doubly-indirect + 1 triply-indirect block pointers

**Directories:**
- Special files containing entries: inode number, name length, name

### Read/Write Operations

**Reading:**
1. Parse superblock
2. Locate inode: `(inode_num - 1) / inodes_per_group`
3. Follow direct/indirect pointers to data blocks
4. Handle indirect blocks recursively

**Writing:**
- Similar traversal + update bitmaps + allocate blocks

### Integration
- Requires VFS layer (see #3)
- Requires block device driver (see #4)

### Complexity Assessment

**Read-only MVP:**
- Superblock parsing: ~100-150 LOC
- Block group descriptors: ~100 LOC
- Inode reading: ~200-250 LOC
- Block pointer resolution: ~150-200 LOC
- Directory parsing: ~150-200 LOC
- VFS integration: ~200-250 LOC
- **Total: ~1100-1500 LOC**

**Full read-write:**
- Add ~500-800 LOC for block allocation, bitmap management, inode creation

**Reference:** [ext2-rs](https://github.com/pi-pi3/ext2-rs) - 2541 LOC, modular implementation

**Estimated effort:** 2-3 weeks for read-only; 4-6 weeks for read-write

---

## 6. Feasible Coreutils

**Complexity:** Varies (Low to High per utility)

### Prerequisites
- VFS implementation (#3)
- Block device (#4) or tmpfs
- Core filesystem syscalls

### Required Syscalls

**Priority 0 (Essential):**
- `openat` (#56) - Open files ⚠️ Missing
- `close` (#57) - ✅ Implemented
- `read` (#63) - ✅ Implemented
- `write` (#64) - ✅ Implemented
- `getdents64` (#61) - Read directories ⚠️ Missing
- `fstatat` (#79) - File metadata ⚠️ Missing

**Priority 1:**
- `mkdirat` (#34), `unlinkat` (#35), `renameat2` (#276), `getcwd` (#17)

**Priority 2:**
- `fchmodat` (#53), `fchownat` (#54), `linkat` (#37), `symlinkat` (#36), `readlinkat` (#78), `utimensat` (#88)

### Easy (1-2 syscalls, minimal logic)

| Command | Primary Syscalls | Notes |
|---------|------------------|-------|
| **cat** | openat, read, write | Concatenate files |
| **head** | openat, read | First N lines |
| **tail** | openat, read | Last N lines |
| **wc** | openat, read | Count lines/words/bytes |
| **echo** | write | Print arguments |
| **mkdir** | mkdirat | Create directories |
| **rmdir** | unlinkat | Remove empty dirs |
| **touch** | openat, utimensat | Create/update timestamps |
| **basename/dirname** | none | Pure path parsing |

### Medium (3-5 syscalls, moderate logic)

| Command | Primary Syscalls | Notes |
|---------|------------------|-------|
| **ls** | openat, getdents64, fstatat | List directory |
| **rm** | unlinkat, getdents64 | Remove files/dirs |
| **cp** | openat, read, write, fstatat, fchmodat | Copy files |
| **mv** | renameat, (fallback: cp+unlink) | Move/rename |
| **ln** | linkat | Hard links |
| **stat** | fstatat | File metadata |
| **chmod** | fchmodat | Change permissions |
| **chown** | fchownat | Change ownership |
| **du** | getdents64, fstatat | Disk usage |

### Hard (complex logic)

| Command | Primary Syscalls | Notes |
|---------|------------------|-------|
| **find** | openat, getdents64, fstatat | Recursive search |
| **sort** | openat, read, write | External sorting |
| **diff** | openat, read | Diffing algorithm |

### Not Feasible Yet
- `chroot`, `date`, `stty`, `who`, `hostname`, `id`, `nice`, `nohup` - Missing subsystems (chroot, RTC, TTY, user/group, scheduler priorities)

### Implementation Order

**Phase 1 (< 2 weeks):**
cat, ls, mkdir, rm, echo, touch, pwd

**Phase 2 (2-4 weeks):**
cp, mv, chmod, stat, head, tail, wc, du

**Phase 3 (4+ weeks):**
find, sort, diff, ln, readlink, dd

**Estimated effort:** 8-12 weeks for filesystem + 15 useful coreutils

---

## 7. QEMU Framebuffer

**Complexity:** Medium

### Current Setup
- QEMU RISC-V with `-nographic -serial stdio`
- Text-only console

### Framebuffer Options

**Option 1: ramfb (Simplest)**
- RAM-based framebuffer via fw_cfg interface
- No PCI driver needed
- QEMU args: `-device ramfb`
- Pros: Simplest, ~100 LOC
- Cons: Poor performance (meant for boot/testing)

**Option 2: bochs-display (Recommended)**
- VGA without legacy cruft
- Clean PCI device with linear framebuffer
- QEMU args: `-device bochs-display`
- PCI BAR 0: framebuffer memory
- PCI BAR 2: MMIO registers for modesetting
- Pros: Clean, good performance, reuses existing PCI infrastructure
- Cons: Requires PCI driver (~200-300 LOC)

**Option 3: virtio-gpu-pci (Most Feature-Rich)**
- Paravirtualized GPU
- QEMU args: `-device virtio-gpu-pci`
- Pros: Best performance, modern
- Cons: Most complex (full VirtIO driver with virtqueues, command buffers)

### Required QEMU Changes
1. Remove `-nographic` from `qemu_wrapper.sh`
2. Add `-device <device>` option
3. Keep `-serial stdio` for console

### Basic Drawing

**Pixel format:** Typically 32-bit XRGB8888
- Red: bits 16-23, Green: 8-15, Blue: 0-7

**Drawing pixel at (x, y):**
```rust
let offset = y * stride + x * bytes_per_pixel;
framebuffer[offset..offset+4].copy_from_slice(&[blue, green, red, 0]);
```

### Recommendation
Start with **bochs-display** - reuses PCI infrastructure, simpler than virtio-gpu, better than ramfb.

**Estimated effort:** 1-2 weeks for bochs-display driver

---

## 8. Port Doom

**Complexity:** High (several weeks)

### What is doomgeneric?
Minimal, highly portable Doom requiring only **5 functions:**
- `DG_Init()` - Initialize
- `DG_DrawFrame()` - Copy 320x200 framebuffer to screen
- `DG_SleepMs()` - Sleep
- `DG_GetTicksMs()` - Get time
- `DG_GetKey()` - Keyboard input

No sound support.

### Requirements

**libc:** ✅ Already have musl libc in userspace

**Syscalls:**

✅ Already implemented:
- `read/write`, `mmap/munmap`, `brk`, `nanosleep`

❌ Missing:
- **Framebuffer access** - Need graphics device (#7)
- **File system** - Need `open/openat/close` for reading WAD files (#3)
- **Keyboard input** - Need input event interface
- **Timing** - Need `clock_gettime` for `DG_GetTicksMs`

### What Needs Implementation

**Major Components:**
1. **Framebuffer** (#7) - VirtIO-GPU or bochs-display driver
2. **File System** (#3) - Basic VFS for reading WAD file
   - Alternative: Embed doom1.wad (shareware, ~4MB) in kernel initially
3. **Keyboard Driver** - VirtIO input or PS/2 keyboard
4. **Timing** - `clock_gettime` syscall

**QEMU Config:**
```bash
qemu-system-riscv64 \
    -machine virt \
    -device virtio-gpu-pci \
    -device virtio-keyboard-pci \
    -serial stdio
```

### Complexity
- Requires framebuffer, filesystem, keyboard, timing
- All pieces must work together
- Debugging rendering issues

**Dependencies:** Items #3 (VFS), #7 (framebuffer), plus keyboard driver

**Estimated effort:** 2-4 weeks once dependencies are complete

---

## 9. Async Network Reception with Interrupts

**Complexity:** Low to Medium

### Current Polling Implementation
`recvfrom()` actively polls network card via `net::receive_and_process_packets()`, returns `EAGAIN` if no data.

**Problem:** Wasteful, prevents true async blocking.

### Proposed Implementation

**Model:** Follow existing timer-based sleep pattern (`kernel/src/processes/timer.rs`)

**Key Components:**

1. **Enable VirtIO Interrupts** (currently disabled)
   - Clear `VIRTQ_AVAIL_F_NO_INTERRUPT` flag in virtqueue.rs:238
   - Configure MSIX vectors in VirtIO driver

2. **Create Waker Queue**
```rust
static RECV_WAITERS: Spinlock<BTreeMap<Port, Vec<Waker>>> = ...;
```

3. **RecvWait Future**
```rust
impl Future for RecvWait {
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        if packet_available(self.port) {
            return Poll::Ready(());
        }
        if !self.registered {
            RECV_WAITERS.lock().insert(self.port, cx.waker().clone());
            self.registered = true;
        }
        Poll::Pending
    }
}
```

4. **PLIC Interrupt Handler**
- Detect VirtIO network interrupt
- Call `handle_network_interrupt()` which processes packets and wakes waiters

### Files to Modify
- `kernel/src/drivers/virtio/net/mod.rs` - Enable interrupts
- `kernel/src/interrupts/plic.rs` - Add VirtIO interrupt source
- `kernel/src/interrupts/trap.rs` - Handle network interrupts
- `kernel/src/net/sockets.rs` - Add waker registration
- `kernel/src/syscalls/linux.rs` - Make `recvfrom()` truly async

### Benefits
- Eliminates wasteful polling
- Threads truly sleep waiting for network data
- Better CPU utilization
- Aligns with existing async infrastructure

**Estimated effort:** 3-5 days

---

## 10. DHCP Client

**Complexity:** Low to Medium

### Protocol Basics

**Four-message handshake (RFC 2131):**
1. **DHCPDISCOVER**: Client broadcasts `0.0.0.0:68` → `255.255.255.255:67`
2. **DHCPOFFER**: Server proposes IP
3. **DHCPREQUEST**: Client accepts offer
4. **DHCPACK**: Server confirms

**Message format:** BOOTP-based (236-byte header + options)

### Recommendation: Userspace Implementation

**Why Userspace:**
- One-time configuration at boot, not performance-critical
- Simpler with standard UDP sockets (already working)
- Follows Unix philosophy
- Easier to iterate without kernel rebuilds

**Kernel Changes Needed:**
- Add broadcast UDP support (`255.255.255.255` destination)
- Bind to `0.0.0.0:68` must work
- Add syscall to configure IP address dynamically
- Make `NETWORK_STACK.ip_addr` mutable (currently hardcoded `10.0.2.15`)

### Minimal Viable Implementation

**Userspace client** (`userspace/src/bin/dhcp.rs`):
```rust
fn main() {
    let socket = UdpSocket::bind("0.0.0.0:68")?;
    socket.set_broadcast(true)?;

    // 1. Send DHCPDISCOVER
    socket.send_to(&discover, "255.255.255.255:67")?;

    // 2. Receive DHCPOFFER
    let offer = parse_offer(&buf)?;

    // 3. Send DHCPREQUEST
    socket.send_to(&request, "255.255.255.255:67")?;

    // 4. Receive DHCPACK
    let ack = parse_ack(&buf)?;

    // 5. Configure interface
    configure_ip(ack.yiaddr)?;  // New syscall
}
```

**Not needed for MVP:**
- Lease renewal, DHCPDECLINE, DHCPRELEASE
- Multiple interfaces
- Full option parsing

### Network Interface Configuration

**Option:** Add simple syscall
```rust
async fn sys_solaya_set_ip(addr_u32: u32) -> Result<isize, Errno> {
    net::set_ip_addr(Ipv4Addr::from(addr_u32));
    Ok(0)
}
```

**Estimated effort:** Kernel changes ~50 LOC; userspace client ~300-400 LOC; 1-2 weeks total

---

## 11. Minimal TCP Implementation

**Complexity:** Medium to High

### What Can Be Omitted

Minimal TCP can safely omit:
- TCP options (ignore incoming, don't send)
- Window scaling (fixed window, e.g., 8192 bytes)
- Congestion control (fixed retransmit timeout)
- Urgent pointer
- Out-of-order buffering (drop initially)
- Advanced RST handling
- Timestamp options, SACK
- Path MTU discovery (fixed MSS, e.g., 1460)

### Connection Establishment (Three-Way Handshake)

**Client:**
1. SYN with random ISN
2. Receive SYN-ACK
3. Send ACK

**TCP Header:** 20 bytes minimum (source/dest port, seq/ack nums, flags, window, checksum)

### Data Transfer
- **Sequence numbers:** 32-bit byte position in stream
- **Acknowledgments:** Cumulative ACK (next expected seq num)
- **Window size:** Fixed receive buffer (e.g., 8192)
- **Retransmission:** Fixed timeout (e.g., 1 second)

### Connection Teardown
Four-way handshake (FIN, ACK, FIN, ACK) - can optimize to three-way.

### Simplified State Machine

**Minimal states:**
- CLOSED, LISTEN, SYN-SENT, SYN-RECEIVED
- ESTABLISHED
- FIN-WAIT-1, FIN-WAIT-2, CLOSE-WAIT, LAST-ACK
- Optional: TIME-WAIT (can skip for faster iteration)

### Integration with Existing Stack

**Reuse:**
- `kernel/src/net/ipv4.rs` - Change protocol 17→6
- `kernel/src/net/checksum.rs` - Same algorithm
- `kernel/src/net/sockets.rs` - Socket management pattern

**New:**
- `kernel/src/net/tcp.rs` - TCP header, parsing, packet creation
- `kernel/src/net/tcp_socket.rs` - Connection state machine
- `kernel/src/net/tcp_sockets.rs` - Global connection table
- Add `FileDescriptor::TcpSocket` variant

**Syscall Modifications:**
- `socket()` accept `SOCK_STREAM`
- Add `listen()`, `accept()`, `connect()`
- Modify `send()`/`recv()` for stream semantics

**Implementation Strategy:**
1. Client-side active open (connect)
2. Basic data transfer
3. Server-side passive open (listen/accept)
4. Graceful close (FIN)
5. Retransmission for reliability

**Estimated effort:** 3-5 weeks

---

## 12. Dynamic Linking

**Complexity:** Medium to High

### Current State
- Fully static linking (`-C target-feature=+crt-static`)
- ELF loader only accepts `FileType::ExecutableFile`
- All binaries embedded in kernel

### ELF Dynamic Linking Basics

**Core Components:**
- **PLT** (Procedure Linkage Table): Indirection for function calls
- **GOT** (Global Offset Table): Function pointers and global variables
- **PT_INTERP**: Path to dynamic linker (e.g., `/lib/ld-linux-riscv64.so.1`)
- **PT_DYNAMIC**: Metadata (needed libraries, symbol tables, relocations)

**RISC-V Relocations:**
- `R_RISCV_CALL_PLT`, `R_RISCV_HI20/LO12_I`, `R_RISCV_RELATIVE`, `R_RISCV_JUMP_SLOT`

### Recommendation: Userspace Dynamic Linker

**Kernel does (minimal):**
1. Detect PT_INTERP segment
2. Load executable + interpreter (ld.so)
3. Map PT_LOAD segments
4. Set up auxv (`AT_PHDR`, `AT_ENTRY`, `AT_BASE`, etc.)
5. Jump to interpreter entry point

**Userspace linker does:**
1. Relocate itself (bootstrap)
2. Parse PT_DYNAMIC
3. Load dependent libraries (DT_NEEDED)
4. Resolve symbols
5. Apply relocations
6. Initialize libraries (.init)
7. Jump to main program

**Advantages:**
- Minimal kernel complexity (~200-300 LOC)
- Dynamic linker uses full libc
- Easier debugging
- Standard approach (Linux/musl/glibc)

### Required Syscalls

✅ Already implemented: `mmap`, `munmap`, `mprotect`, `brk`

❌ Missing: **Filesystem syscalls** (#3)
- `openat`, `close`, `read`, `fstat` - To open and read shared libraries

### Complexity Assessment

**Lines of Code:**
- Kernel changes: ~200-300 LOC (PT_INTERP loading, auxv)
- Userspace dynamic linker: ~1500-2500 LOC

**Minimal Feature Set:**

**Phase 1 (MVP):**
- Kernel loads PT_INTERP and sets auxv
- Basic userspace linker in Rust
- Load single .so (no dependencies)
- Eager symbol resolution only
- 3-4 basic RISC-V relocations

**Phase 2 (Usable):**
- Recursive dependency loading
- Library search paths
- All common relocations
- Multiple libraries

**Phase 3 (Full):**
- Lazy binding (PLT/GOT runtime resolver)
- TLS support
- Performance optimizations

**Main Blocker:** Filesystem support (#3) - currently embeds all binaries

**Estimated effort:** 2-4 weeks once filesystem exists

---

## 13. QEMU Random Device Driver

**Complexity:** Low to Medium

### Device Specification
- VirtIO Device ID: **4**
- PCI Subsystem ID: **4** (differs from network = 1)
- Single virtqueue for entropy requests
- Simpler than network device (no config registers like MAC address)

### Driver Implementation
**Based on existing virtio-net driver:**

**Detection:**
```rust
pub fn is_virtio_rng(device: &PCIDevice) -> bool {
    device.subsystem_id().read() == 4  // RNG subsystem ID
}
```

**Operation:**
1. Reuse virtqueue infrastructure
2. Follow network device initialization pattern
3. Single receive queue with `BufferDirection::DeviceWritable`
4. No transmit queue needed (read-only device)
5. Device writes random bytes into guest buffers

**Simpler than network:**
- No device-specific configuration
- No MAC address or status registers
- Single queue
- No packet headers (raw bytes)

### Linux Auxiliary Vector (auxv)

**AT_RANDOM:**
- Provides pointer to **16 random bytes**
- Constant for process lifetime
- Used for ASLR and runtime security

**Current auxv** (`kernel/src/processes/loader.rs:37-65`):
- `AT_PAGESZ`, `AT_PHDR`, `AT_PHENT`, `AT_PHNUM`, `AT_NULL`

**Required changes:**
1. Allocate 16-byte buffer
2. Fill from virtio-rng during kernel init
3. Add `AT_RANDOM` entry to auxv:
   ```rust
   AT_RANDOM as usize,
   random_bytes_ptr as usize,
   ```

### Integration Points

**Kernel init** (`kernel/src/main.rs:140-150`):
- Enumerate PCI devices
- Detect virtio-rng alongside virtio-net
- Initialize driver, generate entropy pool
- Store in global accessible to loader

**Process creation** (`kernel/src/processes/loader.rs:130`):
- Modify `set_up_arguments()` to include AT_RANDOM
- Generate 16 bytes per process (or use pool)

**Headers:**
- Add `AT_RANDOM` constant to `headers/syscall_types`

### Implementation Order
1. Implement virtio-rng driver
2. Add kernel storage for random bytes
3. Add `AT_RANDOM` constant
4. Modify auxv construction
5. System test to verify `getauxval(AT_RANDOM)` works

**Estimated effort:** 1-2 weeks

---

## Dependencies and Recommended Order

### Phase 1: Foundation (Critical Infrastructure)
1. **#1 - Process IDs/Groups** (Foundation for job control)
2. **#9 - Async Network with Interrupts** (Better performance)
3. **#13 - QEMU Random Device** (Security foundation)

### Phase 2: Storage and Filesystems
4. **#4 - QEMU Block Device Driver** (Prerequisite for filesystems)
5. **#3 - Virtual File System** (Core abstraction)
6. **#5 - ext2 Filesystem** (Persistent storage)
7. **#6 - Coreutils** (User-facing utilities)

### Phase 3: Networking Enhancements
8. **#10 - DHCP Client** (Network configuration)
9. **#11 - Minimal TCP** (Protocol expansion)

### Phase 4: Advanced Features
10. **#2 - Linux Signals** (Process control)
11. **#12 - Dynamic Linking** (Shared libraries)
12. **#7 - Framebuffer** (Graphics foundation)
13. **#8 - Port Doom** (Showcase project)

---

## Clarifying Questions

Before implementation, consider:

1. **Storage Strategy:** For items #4-7 (VFS/filesystem), do you want to start with tmpfs (in-memory) or go directly to block device + ext2?

2. **Framebuffer Choice (#7):** ramfb (simplest), bochs-display (recommended), or virtio-gpu (most complex)?

3. **Dynamic Linking (#12):** Should we prioritize this over other features, or wait until filesystem support is solid?

4. **Signals (#2):** Do you want full signal support including SIGCHLD/job control, or minimal signal delivery first?

5. **Testing Strategy:** Should each major feature include new system tests, or batch testing?

Let me know which items you'd like to prioritize, or if you have questions about any of the research!
