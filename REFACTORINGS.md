# Refactoring Opportunities

Architectural debt and refactoring candidates, ordered by impact. Each item is
independently actionable. Reference by number (e.g. "R-02") in commits and PRs.

---

## High Impact

**R-01 — Introduce a file descriptor table.**
Process has no generic fd table — stdin/stdout/stderr are magic constants checked
in ~10 places across `syscalls/linux.rs` (lines 58, 77, 132, 362, 377). UDP
sockets live in a separate `BTreeMap<UDPDescriptor, SharedAssignedSocket>`. A
proper `FdTable` mapping `i32 → FileDescriptor` (enum of Stdio, Socket, future
File/Pipe) would eliminate all hardcoded fd comparisons and unify socket
management.

**R-02 — Implement munmap.**
`munmap` is a no-op returning `Ok(0)` (`syscalls/linux.rs:347-354`). Every
`mmap` allocation leaks permanently. Implementing real munmap requires
page-table unmapping and feeding pages back to the allocator.

---

## Medium Impact

**R-03 — Replace copy-paste sbi_call variants with a single function.**
`sbi_call.rs` has four near-identical functions (`sbi_call`, `sbi_call_1`,
`sbi_call_2`, `sbi_call_3`) differing only in argument count. A single
`sbi_call(eid, fid, args: [u64; 3])` with unused args zeroed would halve the
file.

**R-04 — Replace unsafe transmute for SbiError / XWRMode / PageStatus.**
`sbi_call.rs:31` uses `core::mem::transmute::<i64, SbiError>()`, and
`page_table_entry.rs` and `page_allocator.rs:82` do the same for enum
conversions. Use `TryFrom` implementations or explicit match arms to make
invalid values a checked error rather than undefined behavior.

**R-05 — Remove hardcoded IP address.**
`net/mod.rs:22` defines `static IP_ADDR: Ipv4Addr = Ipv4Addr::new(10, 0, 2, 15)`
as a compile-time constant. This should be configurable — either from a DHCP
response or a kernel command-line parameter — especially once multiple network
interfaces are possible.

**R-06 — Reduce global statics in networking.**
`net/mod.rs:21-25` has four global `Spinlock<...>` statics: `NETWORK_DEVICE`,
`IP_ADDR`, `ARP_CACHE`, `OPEN_UDP_SOCKETS`. Grouping them into a
`NetworkStack` struct passed by reference would improve testability and make
per-interface state possible.

**R-07 — Introduce UART register constants.**
`uart.rs` uses 11+ magic numbers for register offsets (e.g. `+ 5`, `+ 3`,
`+ 2`, `+ 1`), bit masks (`0b11`, `1 << 7`), and the baud divisor (`592`).
Named constants or a register-field enum would make the code self-documenting.

**R-08 — Make PCI enumeration generic.**
`pci/mod.rs:237-243` hardcodes VirtIO vendor/device ID filtering and only
populates `network_devices`. A generic approach — e.g. a
`register_driver(vendor, device, init_fn)` API — would support block devices,
GPU, etc. without modifying the PCI scanner.

**R-09 — Deduplicate read/write/writev fd validation.**
`read()`, `write()`, and `writev()` in `syscalls/linux.rs` each independently
validate fd ranges with slightly different logic. Once R-01 lands, this
collapses into a single `fd_table.get(fd)?` call, but even before that, a shared
`validate_write_fd(fd)` helper would reduce the three-way duplication.

**R-10 — Create a shared userspace input library.**
`userspace/src/util.rs:10-40` and `userspace/src/bin/udp.rs:30-54` contain
near-identical `read_line()` loops with the same DELETE handling, backspace
escape sequence, and newline logic. The `DELETE` constant is defined twice.
Move the canonical implementation into `userspace/src/util.rs` and have all
programs use it.

**R-11 — Separate Process–Thread ownership.**
`Process` holds `BTreeMap<Tid, ThreadWeakRef>` while each `Thread` holds a
strong `ProcessRef` (`Arc<Spinlock<Process>>`). This bidirectional reference
makes lifetime reasoning difficult. Consider making the scheduler the sole owner
of threads, with processes holding only Tid sets.

---

## Low Impact

**R-12 — Merge system-tests into the workspace.**
`system-tests/` is a standalone workspace and must be built/tested separately.
Merging it into the root workspace (with a `default-members` exclude so
`cargo test` in the kernel still works) would unify dependency management and
simplify CI.

**R-13 — Add a minimal userspace syscall wrapper crate.**
Userspace programs call libc directly. A thin `sentientos-sys` crate with
type-safe wrappers (e.g. `fn send_udp(fd: Fd, buf: &[u8]) -> Result<usize>`)
would reduce boilerplate and catch misuse at compile time.

**R-14 — Replace UART offset arithmetic with an MMIO register struct.**
`uart.rs` constructs each register as `MMIO::new(base + N)`. A single
`UartRegisters` struct with named fields (thr, rbr, ier, fcr, lcr, lsr)
computed from a base address would make register access type-safe and
self-documenting.

**R-15 — Consolidate the ARP path.**
`arp.rs` and the ARP cache in `net/mod.rs` are split across files with the cache
accessed via a global static. Colocating the cache with the ARP protocol handler
in a single `ArpCache` struct would improve cohesion.

**R-16 — Extract virtqueue setup from device init.**
Virtqueue allocation and descriptor ring setup in the VirtIO network driver is
device-independent. Extracting a reusable `VirtQueue::new(index, size)`
constructor prepares for future VirtIO block or console drivers.

**R-17 — Add ethernet frame type dispatch.**
Incoming frames are dispatched by checking the ethertype field inline. A small
dispatch table (`0x0800 → handle_ipv4`, `0x0806 → handle_arp`) would make adding
new L3 protocols trivial.

**R-18 — Use per-CPU scheduler queues.**
The scheduler uses a single global run queue protected by a spinlock. On SMP
this serializes all scheduling decisions. Per-CPU queues with work-stealing
would reduce contention (relevant once the core count grows).

**R-19 — Shrink the ppoll syscall handler.**
`ppoll()` in `syscalls/linux.rs:104-149` mixes fd validation, timeout parsing,
stdin polling, and socket polling in one block. Splitting into
`poll_stdin()` / `poll_socket()` helpers would clarify the logic and make it
easier to extend for new fd types.

**R-20 — Introduce a PacketBuffer / scatter-gather type.**
Network TX currently concatenates `Vec<u8>` slices to build full frames.
A zero-copy scatter-gather list (`&[IoSlice]`) passed down the stack would avoid
intermediate allocations and match how real NICs consume descriptors.
