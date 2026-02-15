# Refactoring Opportunities

Architectural debt and refactoring candidates, ordered by impact. Each item is
independently actionable. Reference by number (e.g. "R-02") in commits and PRs.

---

## Medium Impact

**R-04 — Remove hardcoded IP address.**
`net/mod.rs:22` defines `static IP_ADDR: Ipv4Addr = Ipv4Addr::new(10, 0, 2, 15)`
as a compile-time constant. This should be configurable — either from a DHCP
response or a kernel command-line parameter — especially once multiple network
interfaces are possible.

**R-05 — Create a shared userspace input library.**
`userspace/src/util.rs:10-40` and `userspace/src/bin/udp.rs:30-54` contain
near-identical `read_line()` loops with the same DELETE handling, backspace
escape sequence, and newline logic. The `DELETE` constant is defined twice.
Move the canonical implementation into `userspace/src/util.rs` and have all
programs use it.

**R-06 — Separate Process–Thread ownership.**
`Process` holds `BTreeMap<Tid, ThreadWeakRef>` while each `Thread` holds a
strong `ProcessRef` (`Arc<Spinlock<Process>>`). This bidirectional reference
makes lifetime reasoning difficult. Consider making the scheduler the sole owner
of threads, with processes holding only Tid sets.

---

## Low Impact

**R-07 — Merge system-tests into the workspace.**
`system-tests/` is a standalone workspace and must be built/tested separately.
Merging it into the root workspace (with a `default-members` exclude so
`cargo test` in the kernel still works) would unify dependency management and
simplify CI.

**R-08 — Add a minimal userspace syscall wrapper crate.**
Userspace programs call libc directly. A thin `sentientos-sys` crate with
type-safe wrappers (e.g. `fn send_udp(fd: Fd, buf: &[u8]) -> Result<usize>`)
would reduce boilerplate and catch misuse at compile time.

**R-09 — Use per-CPU scheduler queues.**
The scheduler uses a single global run queue protected by a spinlock. On SMP
this serializes all scheduling decisions. Per-CPU queues with work-stealing
would reduce contention (relevant once the core count grows).

**R-10 — Introduce a PacketBuffer / scatter-gather type.**
Network TX currently concatenates `Vec<u8>` slices to build full frames.
A zero-copy scatter-gather list (`&[IoSlice]`) passed down the stack would avoid
intermediate allocations and match how real NICs consume descriptors.
