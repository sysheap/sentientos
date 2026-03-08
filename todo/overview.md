# Solaya OS - Future Implementation Todos

This document contains research summaries for planned future enhancements. Each item includes implementation complexity, requirements, and key design decisions.

## Table of Contents

1. [QEMU Block Device Driver](#1-qemu-block-device-driver)
2. [ext2 Filesystem](#2-ext2-filesystem)
4. [QEMU Framebuffer](#4-qemu-framebuffer)
5. [Port Doom](#5-port-doom)
6. [Minimal TCP Implementation](#6-minimal-tcp-implementation)
7. [Dynamic Linking](#7-dynamic-linking)
8. [QEMU Random Device Driver](#8-qemu-random-device-driver)

---

## 1. QEMU Block Device Driver

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

## 2. ext2 Filesystem

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
- Requires VFS layer (done)
- Requires block device driver (see #1)

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

## 4. QEMU Framebuffer

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

## 5. Port Doom

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
- **Framebuffer access** - Need graphics device (#4)
- **File system** - VFS is done; need persistent storage (#1 block device + #2 ext2) for WAD files
- **Keyboard input** - Need input event interface
- **Timing** - Need `clock_gettime` for `DG_GetTicksMs`

### What Needs Implementation

**Major Components:**
1. **Framebuffer** (#4) - VirtIO-GPU or bochs-display driver
2. **File System** - VFS with tmpfs/procfs is done. Need persistent storage for WAD files.
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

**Dependencies:** Items #4 (framebuffer), plus keyboard driver. VFS is done.

**Estimated effort:** 2-4 weeks once dependencies are complete

---

## 6. Minimal TCP Implementation

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

## 7. Dynamic Linking

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

✅ Implemented: **Filesystem syscalls**
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

**Main Blocker:** Persistent filesystem (ext2 + block device) - currently embeds all binaries

**Estimated effort:** 2-4 weeks once filesystem exists

---

## 8. QEMU Random Device Driver

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

## Clarifying Questions

Before implementation, consider:

1. **Framebuffer Choice (#4):** ramfb (simplest), bochs-display (recommended), or virtio-gpu (most complex)?

2. **Dynamic Linking (#9):** Should we prioritize this over other features, or wait until persistent filesystem support is ready?

3. **Testing Strategy:** Should each major feature include new system tests, or batch testing?

Let me know which items you'd like to prioritize, or if you have questions about any of the research!
