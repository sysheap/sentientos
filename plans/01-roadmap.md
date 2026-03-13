# Solaya to Linux-Compatible OS: Comprehensive Roadmap

## 1. Current State

For a detailed inventory of the current system, see `doc/ai/OVERVIEW.md` and the linked subsystem documentation:

- **Syscalls:** `doc/ai/SYSCALLS.md` -- full list of implemented syscalls and dispatch architecture
- **Memory:** `doc/ai/MEMORY.md` -- page allocator, page tables, heap
- **Processes:** `doc/ai/PROCESSES.md` -- process/thread lifecycle, scheduler
- **Filesystem:** `doc/ai/FS.md` -- VFS layer, tmpfs, procfs, devfs, ext2
- **Networking:** `doc/ai/NETWORKING.md` -- UDP, TCP, sockets
- **Interrupts:** `doc/ai/INTERRUPTS.md` -- trap handling, PLIC, timer
- **Drivers:** `doc/ai/DRIVERS.md` -- VirtIO drivers, PCI
- **Architecture:** `doc/ai/ARCHITECTURE.md` -- boot sequence, data structures, memory layout

The roadmap below describes what's missing and the order to build it.

---

## 2. Phase-Based Roadmap

### Phase 1: Foundation Hardening (3-6 months)

**Goal:** Make the existing subsystems robust enough to be a reliable base. No new features -- fix what exists.

#### 1.1 Copy-on-Write Fork
The current fork copies the entire address space page-by-page. This is the single largest performance bottleneck for shell usage. Every fork()+exec() cycle (which is how every command runs) copies potentially megabytes of memory that gets immediately thrown away by execve.

- Implement page fault handler for write faults on CoW pages
- Track reference counts on physical pages
- Mark shared pages as read-only, trap on write, copy-then-remap
- This touches: page_tables.rs, process.rs (fork_address_space), trap.rs (exception handler)

#### 1.2 Demand Paging and Lazy Allocation
Currently mmap immediately allocates and zeros all pages. Real Linux uses lazy allocation (allocate on first access).

- Implement page fault handler for missing pages
- Support MAP_ANONYMOUS with lazy allocation
- Support MAP_PRIVATE file-backed mappings (needed for mmap-based file I/O and shared libraries)
- This unlocks running larger programs without exhausting memory

#### 1.3 Page Allocator Improvements
The current allocator is a linear-scan bitmap protected by a single global spinlock. This becomes a bottleneck under SMP.

- Replace linear scan with free-list or buddy allocator
- Per-CPU page caches (like Linux's per-CPU page frame cache)
- Track memory statistics for /proc/meminfo

#### 1.4 Robust VFS Layer
The current VFS is minimal. Files lack metadata (ownership, permissions, timestamps).

- Add uid/gid/mode/timestamps to VfsNode trait
- Implement symlink support (NodeType::Symlink + readlink)
- Implement hard links (link count tracking)
- Implement rename
- Implement O_APPEND, O_EXCL, O_CREAT flag handling properly
- Add statfs/fstatfs support

#### 1.5 Process Model Gaps
- Implement proper credential tracking (uid/gid/euid/egid/saved-set-uid)
- Implement setuid/setgid/setreuid/setregid/setresuid/setresgid
- Implement getgroups/setgroups (supplementary groups)
- Implement getrlimit/setrlimit/prlimit64 (resource limits, even if initially unenforced)
- Implement proper prctl (at least PR_SET_NAME)

#### 1.6 Missing Critical Syscalls
Syscalls that nearly every real program needs and are currently missing:

- dup / dup2 (currently only dup3)
- pread64 / pwrite64 (positional I/O without seeking)
- access (currently only faccessat)
- truncate / ftruncate
- rename / renameat / renameat2
- link / linkat / symlink / symlinkat
- readlink (make readlinkat functional)
- chmod / fchmod / fchmodat
- chown / fchown / fchownat
- uname (returns system information -- essential for configure scripts and many programs)
- getrandom / getentropy
- sysinfo (memory/uptime info)
- getrusage

### Phase 2: POSIX Core (6-12 months)

**Goal:** Run GNU coreutils, BusyBox, and common shell utilities unmodified.

#### 2.1 Complete Signal Subsystem
- SA_SIGINFO support (3-argument signal handlers with siginfo_t)
- Real-time signals (signals 32-64)
- sigqueue for real-time signal delivery with data
- signalfd (signals via file descriptor -- needed by systemd and many daemons)
- sigsuspend, sigwaitinfo, sigtimedwait
- Proper POSIX semantics: signal delivery to specific thread in multi-threaded process

#### 2.2 I/O Multiplexing
This is the single most important missing subsystem for running real server software.

- select / pselect6 -- legacy but still widely used
- poll -- ppoll exists but poll itself is commonly called
- epoll_create1 / epoll_ctl / epoll_wait -- required by every modern event loop (libuv, tokio, Go runtime, nginx, Redis)
- Architecture: epoll needs a kernel-side interest list and readiness notification from each file descriptor type (sockets, pipes, TTYs, timers, signals via signalfd, etc.)

#### 2.3 Pseudo-Terminals
Required for SSH, screen/tmux, expect, and any program that needs a terminal but is not directly attached to hardware.

- Implement /dev/ptmx (pty master) and /dev/pts/* (pty slaves)
- posix_openpt / grantpt / unlockpt / ptsname
- Controlling terminal association
- Session leaders and controlling terminal assignment on open

#### 2.4 Unix Domain Sockets
Required by many programs for local IPC (X11, DBus, systemd, Docker, most language runtimes).

- AF_UNIX socket support (SOCK_STREAM + SOCK_DGRAM)
- Abstract namespace sockets
- SCM_RIGHTS (file descriptor passing)
- SCM_CREDENTIALS (credential passing)
- socketpair

#### 2.5 File Locking
- flock (BSD-style whole-file locks)
- fcntl F_SETLK/F_SETLKW/F_GETLK (POSIX record locks)
- F_OFD_SETLK (open file description locks)
- Needed by databases, package managers, any multi-process application

#### 2.6 Users, Groups, and Permissions
- Enforce file permission checks on open/read/write/exec
- Implement setuid/setgid execution
- Process credential management (already started in Phase 1)
- /etc/passwd and /etc/group parsing (userspace concern, but kernel must support the underlying model)

#### 2.7 Job Control Completion
Solaya has basic job control (pgid, sid, SIGTSTP/SIGCONT, foreground group). Remaining work:

- SIGTTOU for background writes
- Proper orphaned process group handling
- Controlling terminal revocation

#### 2.8 Timer and Clock Subsystem
- timer_create / timer_settime / timer_delete (POSIX timers)
- timerfd_create / timerfd_settime (timer via file descriptor)
- clock_getres
- CLOCK_MONOTONIC, CLOCK_REALTIME, CLOCK_BOOTTIME support
- eventfd (simple inter-process signaling -- heavily used by epoll-based event loops)

### Phase 3: Linux Essentials (12-24 months)

**Goal:** Run complex server software (nginx, Redis, PostgreSQL, Node.js, Python, Go programs).

#### 3.1 Advanced Futex Operations
Current futex only supports FUTEX_WAIT and FUTEX_WAKE. Full support requires:

- FUTEX_WAIT_BITSET / FUTEX_WAKE_BITSET
- FUTEX_REQUEUE / FUTEX_CMP_REQUEUE (used by pthread_cond implementations)
- FUTEX_WAKE_OP (atomic wake-and-modify)
- PI-futexes (priority inheritance -- needed for real-time applications)
- Robust futex list handling (currently stubbed)

#### 3.2 Writable Disk Filesystem
Solaya's ext2 is read-only and loads the entire directory tree into memory at mount time. A real system needs:

- Writable ext2 (or ext4 -- ext2 is simpler and sufficient for many use cases)
- Block cache / page cache
- Write-back and fsync support
- mount / umount syscalls
- /etc, /var, /tmp as real writable filesystems
- Consider: starting with a proper tmpfs that supports persistence to disk, or implementing ext2 write support

#### 3.3 procfs and sysfs
Nearly every Linux tool reads from /proc or /sys. Current procfs has only /proc/version.

**Critical /proc entries:**
- /proc/self and /proc/[pid]/ symlinks
- /proc/[pid]/status, stat, maps, cmdline, fd/, cwd, exe, root
- /proc/cpuinfo, meminfo, loadavg, uptime, stat
- /proc/mounts, /proc/filesystems
- /proc/sys/ (sysctl interface)
- /proc/net/ (tcp, udp, unix, etc.)

**sysfs:**
- /sys/class/ (device classes)
- /sys/devices/ (device hierarchy)
- Basic structure needed by udev and many programs that probe hardware

#### 3.4 Namespaces
Linux namespaces are the foundation of containers. Implementation order by importance:

1. **PID namespace** -- separate PID number spaces
2. **Mount namespace** -- separate filesystem trees
3. **Network namespace** -- separate network stacks
4. **User namespace** -- separate UID/GID mappings
5. **UTS namespace** -- separate hostname
6. **IPC namespace** -- separate System V IPC
7. **Cgroup namespace** -- separate cgroup views

Each namespace adds complexity to clone() flags and unshare().

#### 3.5 cgroups v2
Resource management for process groups:

- CPU controller (cpu.max, cpu.weight)
- Memory controller (memory.max, memory.current, OOM)
- I/O controller (io.max)
- PID controller (pids.max)
- cgroupfs filesystem
- Process membership tracking

#### 3.6 Capabilities
Replace the all-or-nothing root/non-root model:

- Capability sets (effective, permitted, inheritable, bounding, ambient)
- capget / capset syscalls
- File capabilities (stored in xattrs)
- Capability-aware execve

#### 3.7 seccomp-bpf
Many security-sensitive programs (Chrome, Docker, systemd) require seccomp:

- seccomp(SECCOMP_SET_MODE_FILTER) with BPF programs
- BPF program verifier and interpreter
- Integration with ptrace

#### 3.8 io_uring
The modern async I/O interface, increasingly required by high-performance applications:

- Submission/completion ring setup
- Supported opcodes: read/write, openat, close, connect, accept, send/recv, poll, timeout
- This is complex but increasingly expected by applications

### Phase 4: Full Networking (12-18 months, overlaps with Phase 3)

**Goal:** Run real network services and clients.

#### 4.1 TCP Completeness
Current TCP is minimal (no window management, no congestion control). Required:

- Sliding window with proper flow control
- Congestion control (at minimum: Reno or CUBIC)
- SACK (Selective Acknowledgment)
- Window scaling (RFC 7323)
- Keep-alive
- TIME_WAIT state management
- Proper retransmission with exponential backoff
- Nagle's algorithm (TCP_NODELAY)
- TCP_CORK
- Urgent data (MSG_OOB)

#### 4.2 IPv6
Increasingly required, especially for modern applications:

- ICMPv6 (Neighbor Discovery replaces ARP)
- IPv6 routing
- Dual-stack socket support (AF_INET6 with v4-mapped addresses)

#### 4.3 ICMP
- Ping (ICMP echo request/reply)
- Destination unreachable messages
- Used by many network diagnostic tools and protocols

#### 4.4 Routing
- Routing table with configurable routes
- Default gateway
- Netlink interface for route management (ip route)
- /proc/net/route

#### 4.5 Netfilter
The Linux firewall framework:

- nftables or iptables hooks
- Connection tracking (conntrack)
- NAT (SNAT/DNAT/masquerade)
- Packet filtering chains (INPUT/OUTPUT/FORWARD)
- This is a very large subsystem; even a basic implementation is significant

#### 4.6 DNS and Higher-Level Protocol Support
- While DNS resolution is a userspace concern (libc), the kernel must support the required syscalls
- Raw sockets (AF_PACKET) for network diagnostic tools
- Multicast (IGMP)

### Phase 5: Device Model and Drivers (12-18 months, overlaps with Phase 3-4)

**Goal:** Support real hardware and the Linux device model.

#### 5.1 Linux Driver Model
- kobject/kset/ktype hierarchy
- Bus/device/driver framework
- sysfs auto-population from device tree
- Hotplug support and uevent notifications
- Platform device/driver matching

#### 5.2 Block Layer
- Generic block device layer (bio/request queue)
- Block I/O scheduler
- Partitioning (GPT/MBR parsing)
- Loop devices
- Device mapper (optional but important for LVM/encryption)

#### 5.3 Storage
- NVMe driver (important for real hardware)
- AHCI/SATA support
- SCSI subsystem (even VirtIO-SCSI uses it)

#### 5.4 Display and Input
- DRM (Direct Rendering Manager) framework
- KMS (Kernel Mode Setting)
- Input subsystem (evdev interface)
- Linux input event protocol (/dev/input/event*)

#### 5.5 x86_64 Architecture Port
Solaya is currently RISC-V only. x86_64 is essential for running on real hardware and cloud VMs.

- Boot protocol (UEFI or multiboot2)
- x86 page tables (4-level, eventually 5-level)
- APIC/IOAPIC interrupt handling
- x86 context switch (TSS, IST, syscall/sysret)
- ACPI parsing (for device enumeration, power management, CPU topology)
- x86 timer sources (HPET, TSC, APIC timer)
- This is a substantial effort; the arch/ crate's abstraction helps but the differences are deep

#### 5.6 CPU Bug Mitigations
RISC-V currently has very few known CPU errata requiring software mitigations, so this is not a concern for the initial RISC-V target. When the x86_64 port begins (5.5), CPU bug mitigations become mandatory before any production use.

Required mitigations for x86_64:
- **Spectre v1/v2:** Retpolines for indirect branch prediction attacks; IBRS/STIBP where available
- **Meltdown:** KPTI (Kernel Page Table Isolation) -- separate page tables for user and kernel mode
- **Spectre v4 (SSBD):** Speculative Store Bypass Disable for sensitive code paths
- **MDS/TAA/MMIO:** Microarchitectural data sampling mitigations (buffer clearing on context switch)
- **L1TF:** L1 Terminal Fault mitigations for virtualization scenarios
- **SRSO/Inception (AMD):** Return address prediction mitigations

All mitigations are based on public CPU vendor advisories (Intel/AMD errata documents, architecture manuals, and published mitigation guides), fully compatible with MIT licensing.

**Strategy:** Defer implementation until the x86_64 port is underway. On first boot, detect CPU model and applicable errata via CPUID. Enable mitigations conditionally. The tracing infrastructure planned in Phase 6 (perf_event_open in 6.2 and eBPF in 6.3) will be valuable for measuring mitigation performance overhead.

### Phase 6: Advanced Features (18-36 months)

**Goal:** Feature parity with a production Linux kernel for developer/server workloads.

#### 6.1 ptrace
Required by debuggers (gdb, lldb, strace) and security tools:

- PTRACE_TRACEME, PTRACE_ATTACH, PTRACE_DETACH
- PTRACE_PEEKTEXT/POKETEXT, PTRACE_PEEKUSER/POKEUSER
- PTRACE_GETREGS/SETREGS
- PTRACE_SYSCALL, PTRACE_SINGLESTEP
- PTRACE_CONT
- PTRACE_SETOPTIONS (fork/exec/clone tracking)
- waitpid integration with trace events

#### 6.2 perf_event_open
Performance monitoring:

- Hardware performance counters
- Software events (context switches, page faults, etc.)
- Sampling and counting modes
- perf_event file descriptor interface

#### 6.3 eBPF
The Swiss Army knife of modern Linux:

- BPF verifier
- JIT compiler for BPF bytecode
- BPF map types (hash, array, ring buffer)
- Attachment points: kprobes, tracepoints, XDP, cgroup, socket
- bpf() syscall

#### 6.4 Audit Subsystem
- System call auditing
- Audit rules and filters
- Audit event logging

#### 6.5 IPC Subsystem
System V IPC (still used by some legacy applications):

- Shared memory (shmget/shmat/shmdt/shmctl)
- Semaphores (semget/semop/semctl)
- Message queues (msgget/msgsnd/msgrcv/msgctl)
- POSIX equivalents (shm_open, sem_open, mq_open)

#### 6.6 Filesystem Notifications
- inotify (inotify_init1, inotify_add_watch, inotify_rm_watch)
- fanotify (more powerful, used by antivirus and backup tools)
- Required by file managers, build systems, IDE file watchers

---

## 3. Syscall Coverage Plan

Linux on riscv64 has approximately 320 syscalls (the riscv64 ABI uses the "new" syscall numbers). Here they are grouped by priority.

### Tier 1: Critical for Basic Programs (~80 syscalls)

These are needed to run a shell, coreutils, and simple C programs. Solaya already implements most of these.

**Already implemented:**
read, write, openat, close, fstat, newfstatat, lseek, mmap, mprotect,
munmap, brk, ioctl, readv, writev, pipe2, dup3, nanosleep, getpid,
clone, execve, exit, wait4, kill, fcntl, getcwd, chdir, mkdirat,
unlinkat, getdents64, exit_group, set_tid_address, clock_gettime,
clock_nanosleep, gettid, futex, set_robust_list, faccessat, ppoll,
readlinkat, statx, rt_sigaction, rt_sigprocmask, rt_sigreturn,
sigaltstack, tgkill, tkill, getuid, geteuid, getgid, getegid,
getppid, getpgid, setpgid, setsid, getsid, umask, madvise, prctl,
fadvise64

**Missing -- high priority:**
pread64, pwrite64 (positional I/O, used by SQLite and databases),
uname (system info, used by every configure script),
access or faccessat2 (file access check, used by shells),
truncate and ftruncate (file truncation),
renameat and renameat2 (rename files),
linkat and symlinkat (link creation),
fchmod and fchmodat (permission changes),
fchown and fchownat (ownership changes),
getrandom (random bytes, used by crypto, TLS, language runtimes),
sysinfo (memory/uptime info),
getrusage (resource usage),
prlimit64 (resource limits),
dup via dup3 (fd duplication),
sendfile (efficient file-to-socket transfer),
clone3 (modern clone interface)

### Tier 2: Needed for Common Tools and Libraries (~60 syscalls)

These are needed by common tools (grep, find, tar, git), language runtimes (Python, Node.js, Go, Rust), and libraries (glibc, musl).

epoll_create1, epoll_ctl, epoll_wait (event loop for nginx, Node.js, Go, Redis),
select and pselect6 (legacy I/O multiplexing),
getsockopt (missing, needed for real networking),
recvmsg and sendmsg (scatter/gather I/O with ancillary data),
socketpair (pair of connected sockets, used by bash),
eventfd and eventfd2 (lightweight signaling, epoll companion),
timerfd_create and timerfd_settime (timer via fd, epoll companion),
signalfd and signalfd4 (signal via fd, epoll companion),
inotify_init1, inotify_add_watch, inotify_rm_watch (file change notification),
mount and umount2 (filesystem mounting),
statfs and fstatfs (filesystem info),
sync, fsync, fdatasync (data persistence),
flock (file locking),
fallocate (file space preallocation),
memfd_create (anonymous fd-backed memory),
mlock, munlock, mlockall, munlockall (page pinning),
mremap (remap pages),
mincore (page residency check),
sched_yield (cooperative scheduling),
sched_getaffinity, sched_setaffinity (CPU pinning),
sched_getscheduler, sched_setscheduler (scheduling policy),
setitimer, getitimer (interval timers),
timer_create, timer_settime, timer_delete, timer_gettime, timer_getoverrun,
capget, capset (capabilities),
personality (execution domain),
waitid (extended wait),
vfork (lightweight fork),
getcpu (which CPU am I on),
syslog (kernel log buffer),
setns, unshare (namespace operations),
pivot_root (change root filesystem)

### Tier 3: Needed for Servers and Containers (~60 syscalls)

These are needed for running production server software and container workloads.

io_uring_setup, io_uring_enter, io_uring_register (async I/O),
seccomp (syscall filtering),
bpf (eBPF),
ptrace (debugging),
perf_event_open (performance monitoring),
copy_file_range (efficient file copying),
splice, tee, vmsplice (zero-copy I/O),
process_vm_readv, process_vm_writev (cross-process memory access),
kcmp (compare kernel objects),
userfaultfd (userspace page fault handling),
membarrier (memory barrier across CPUs),
pkey_alloc, pkey_free, pkey_mprotect (memory protection keys),
faccessat2 (extended access check),
openat2 (extended open),
close_range (bulk close),
pidfd_open, pidfd_send_signal (process fd operations),
epoll_pwait2 (extended epoll wait),
shmget, shmat, shmdt, shmctl (SysV shared memory),
semget, semop, semctl (SysV semaphores),
msgget, msgsnd, msgrcv, msgctl (SysV message queues),
mq_open, mq_timedsend, mq_timedreceive, mq_notify, mq_unlink (POSIX MQs),
quotactl (filesystem quotas),
keyctl, add_key, request_key (kernel keyring),
landlock_create_ruleset, landlock_add_rule, landlock_restrict_self (sandboxing)

### Tier 4: Rarely Used / Legacy (~100+ syscalls)

These are arch-specific, deprecated, or used by very few programs:

riscv_hwprobe (RISC-V hardware probing),
kexec_load, kexec_file_load (hot kernel reload),
reboot (system reboot),
init_module, finit_module, delete_module (kernel modules),
acct (process accounting),
adjtimex, clock_adjtime (clock adjustment for NTP),
ioprio_set, ioprio_get (I/O priority),
lookup_dcookie (profiling),
remap_file_pages (deprecated),
migrate_pages, move_pages, mbind, get_mempolicy, set_mempolicy (NUMA),
swapon, swapoff (swap management),
settimeofday (set system time),
sethostname, setdomainname (hostname management),
and many more arch-specific and obsolete calls.

### Implementation Strategy

**Phase 1-2 focus:** Cover Tier 1 completely and begin Tier 2, targeting ~140 syscalls total. This is enough to run busybox, coreutils, dash, bash, Python, and basic network clients.

**Phase 3-4 focus:** Complete Tier 2 and begin Tier 3, targeting ~200 syscalls total. This is enough to run nginx, Redis, PostgreSQL, Node.js, Go programs, and basic container workloads.

**Phase 5-6 focus:** Complete Tier 3 and selectively implement Tier 4 as needed, targeting ~250+ syscalls. This matches gVisor's coverage (274/350 on amd64).

---

## 4. Key Architectural Decisions

### 4.1 Async Syscall Model -- Does It Scale?

**Current design:** Every syscall is an async Rust future. Blocking syscalls (read from TTY, network recv, futex wait, sleep) yield via Poll::Pending, and a per-thread waker resumes them. Non-blocking syscalls complete synchronously in a single poll.

**Strengths:**
- Elegant integration with Rust's async/await
- No kernel thread per blocked syscall (like Linux's workqueues)
- Natural fit for I/O multiplexing (epoll can poll multiple futures)
- Already works well for the kernel task system (TCP connection state machine, network RX, block I/O)

**Concerns for scaling:**
- **epoll interaction:** epoll needs to register interest across file descriptor types. The current waker-per-thread model needs extending so that multiple file descriptors can wake the same epoll wait.
- **Signal interruption:** When a signal arrives during a blocked syscall, the task must be dropped and EINTR returned. This already works but becomes more complex with io_uring.
- **Memory overhead:** Each pending syscall stores a boxed future. For thousands of concurrent operations, this adds up. Consider arena allocation for futures.
- **Priority inversion:** The current scheduler has no priority mechanism. A compute-bound thread can starve I/O completion. CFS or at least basic priority levels are needed.

**Recommendation:** The async model is sound and should be kept. It is architecturally similar to how io_uring works in Linux (submission queue / completion queue). The main work is adding the epoll layer that can wait on multiple heterogeneous waker sources.

### 4.2 x86_64 Architecture Support

**Required changes:**
- New arch/src/x86_64/ directory with: page tables (4-level), GDT/IDT/TSS, APIC/IOAPIC, syscall/sysret entry, context switch, TLB management
- Boot protocol: UEFI (via Limine or similar bootloader) or multiboot2
- ACPI parsing for device enumeration
- x86 has a different syscall ABI (different register conventions, different syscall numbers)

**Strategy:** The existing arch/ crate abstraction is the right pattern. Define traits for the hardware abstraction and implement them per-arch. The kernel code above arch/ should be architecture-independent. Current RISC-V-specific code in the kernel (satp handling, ecall) needs to be pushed down into arch/.

**Risk:** The current kernel has some RISC-V assumptions baked in (Sv39 address space layout, SBI-based boot, RISC-V trap model). These need to be audited and abstracted.

### 4.3 Memory Model

**Near-term (Phases 1-3):**
- CoW fork (highest priority)
- Demand paging
- Page cache for file I/O
- Buddy allocator or slab allocator
- These are standard and well-understood

**Medium-term (Phases 4-5):**
- Huge pages (2MB) for performance-sensitive workloads
- mmap file-backed mappings with page cache integration
- Shared memory (MAP_SHARED, shmem)
- Swap (even a basic swap to block device helps with memory pressure)

**Long-term (Phase 6):**
- NUMA awareness (topology detection, per-node allocators, migration)
- KSM (Kernel Same-page Merging) -- useful for VMs
- THP (Transparent Huge Pages) -- automatic huge page promotion
- Memory cgroups with OOM control

### 4.4 Locking Strategy

The current kernel uses a single Spinlock type everywhere. This works for a small system but creates problems at scale:

- **Reader-writer locks:** Many data structures (process table, VFS) are read-heavy. RwLock would improve SMP scalability.
- **Lock ordering:** No formal lock ordering exists. As the kernel grows, deadlocks will become a real risk. Document and enforce a lock hierarchy.
- **Sleeping locks:** Spinlocks are only appropriate when hold times are short. For operations that might block (disk I/O, page fault handling), mutex-style locks that can sleep are needed.
- **RCU:** For read-mostly data structures (routing table, mount table, process list), RCU eliminates reader-side locking entirely. This is one of Linux's key scalability innovations.

### 4.5 Scheduler

The current scheduler is a simple FIFO run queue per CPU. For Linux compatibility:

- **CFS (Completely Fair Scheduler):** Linux's default. Uses a red-black tree of virtual runtimes. Essential for good interactive performance and fairness.
- **Priority levels:** nice values (-20 to 19), at minimum
- **Scheduling classes:** SCHED_OTHER (CFS), SCHED_FIFO, SCHED_RR (real-time)
- **CPU affinity:** sched_setaffinity/sched_getaffinity
- **Load balancing:** Migrate threads between CPUs when loads are uneven

---

## 5. Risk Assessment

### 5.1 Highest-Risk / Most Time-Consuming Areas

**1. TCP/IP Stack (Very High Risk)**

A fully conformant TCP implementation is one of the hardest parts of any OS. Linux's net/ipv4/tcp*.c is approximately 30,000 lines of code accumulated over 30 years. The current Solaya TCP lacks congestion control (this alone is a research field), window management, proper state machine (11 states with edge cases), and retransmission with exponential backoff.

gVisor learned this the hard way: their documentation states that "network stacks are complex and writing a new one comes with many challenges, mostly related to application compatibility and performance." gVisor's netstack is one of its largest subsystems.

**Mitigation:** Solaya does not use third-party runtime dependencies. The TCP stack must be written from scratch, but it can follow well-documented RFCs (793, 5681, 6298, 7323) and reference existing implementations for edge cases. Start with a minimal but correct implementation (Reno congestion control, basic SACK) and iterate.

**2. Filesystem Complexity (High Risk)**

Linux's ext4 is approximately 50,000 lines of code. Even ext2 write support requires careful metadata management, crash consistency, and journaling for safety.

**Mitigation:** For a long time, tmpfs + read-only ext2 may be sufficient. When writable storage is needed, consider starting with a simple log-structured filesystem, or use virtio-fs/9p to share a host filesystem (much less kernel code).

**3. epoll Implementation (Medium-High Risk)**

epoll is architecturally pervasive -- every file descriptor type needs to support readiness notifications. This means changes to: pipes, sockets (UDP, TCP, Unix), TTYs, timers, signals, inotify, and every future file descriptor type.

**Mitigation:** Design the readiness notification trait early and implement it in the VFS/FD layer so that all new file types automatically get epoll support.

**4. Namespaces and cgroups (High Risk)**

These touch every subsystem. PID namespaces require PID translation throughout the kernel. Mount namespaces require copy-on-write mount tables. Network namespaces require a complete per-namespace network stack instance.

**Mitigation:** Implement namespaces incrementally. Start with UTS (trivial), then mount, then PID. Skip network namespaces until the network stack is mature.

**5. x86_64 Port (Medium Risk, High Effort)**

Not technically risky (x86_64 is well-documented) but requires a large amount of work: boot, interrupt handling, page tables, ACPI, I/O APIC, timer sources, and every driver needs an x86 path.

**Mitigation:** Use an existing bootloader (Limine). Target QEMU first (which simplifies ACPI and device enumeration). Port incrementally: boot then serial output then interrupts then memory then scheduling then syscalls.

### 5.2 Medium-Risk Areas

**6. Signal Semantics (Medium Risk)**

Linux signal semantics are notoriously complex, especially for multi-threaded processes. Edge cases: signal delivery during fork, exec, exit; thread group signal delivery; signal handler reentrancy; sigaltstack exhaustion.

**7. Permissions and Security Model (Medium Risk)**

Real programs expect permission checks to work correctly. Getting setuid/setgid right is security-critical. Capabilities add complexity but are required by containers.

**8. /proc and /sys Completeness (Low Risk, High Effort)**

Not technically challenging but extremely tedious. Every tool reads different /proc files. Incompatibilities here cause subtle failures.

### 5.3 Where Will Most Time Be Spent?

Based on experience from gVisor, Unikraft, and other Linux-compatible projects:

1. **Fixing compatibility edge cases** -- 40% of total effort. The "last 10%" of each syscall's semantics takes 50% of the implementation time. Programs depend on exact errno values, exact flag combinations, exact behavior on edge cases (empty buffers, interrupted syscalls, races).

2. **Networking** -- 20% of effort. TCP alone is a massive investment.

3. **Filesystem** -- 15% of effort. VFS, page cache, and at least one writable filesystem.

4. **Testing and debugging** -- 15% of effort. Every new syscall needs tests. System-level testing (running real programs) finds integration bugs that unit tests miss.

5. **Architecture and plumbing** -- 10% of effort. Scheduler, memory management, SMP.

### 5.4 Lessons from Other Projects

**gVisor (Google):**
- Implemented 274 of ~350 syscalls on amd64
- "Most language runtimes and libraries that call an unimplemented syscall have fallback code to use an alternative"
- Regression tests against Python, Java, Node.js, PHP, Go runtimes catch compatibility issues early
- Networking was the hardest part and a continuous source of compatibility issues

**Unikraft:**
- Found that only ~224 syscalls are needed statically, and in practice far fewer
- Their 160+ syscalls are sufficient to run Redis, SQLite, NGINX, and several language runtimes
- Binary compatibility mode has significant overhead from user/kernel TLS switching
- PIE (Position-Independent Executable) requirement limits which binaries can run

**OSv:**
- Focused on running a single application per VM, avoiding multi-process complexity
- This dramatically reduces the surface area (no fork, no exec, no multi-user)

**Key takeaway:** The path to running real programs is: (1) implement the top 50 syscalls correctly, (2) try to run a target program, (3) fix what breaks, (4) repeat. Theoretical completeness matters less than practical compatibility with specific target applications.

---

## 6. Milestone Targets

| Milestone | Target | Key Indicator |
|-----------|--------|---------------|
| M1 | CoW fork + demand paging | Fork is 10x faster; memory usage drops |
| M2 | Run BusyBox unmodified | busybox ls, busybox cat, busybox grep work |
| M3 | Run bash (not just dash) | bash starts and runs scripts |
| M4 | epoll works | Simple event loop program works |
| M5 | Run Python interpreter | python3 -c "print('hello')" works |
| M6 | Run Redis | redis-server starts, redis-cli can GET/SET |
| M7 | Run nginx | nginx serves static files over HTTP |
| M8 | Container basics | unshare + chroot + basic namespaces |
| M9 | x86_64 boot | Kernel boots on QEMU x86_64 |
| M10 | Self-hosting | Compile a Rust program on Solaya |

Each milestone gates real progress. M1-M3 are Phase 1-2. M4-M7 are Phase 3-4. M8-M10 are Phase 5-6.

---

## Sources

- [gVisor Linux/amd64 Syscall Compatibility](https://gvisor.dev/docs/user_guide/compatibility/linux/amd64/)
- [gVisor Application Compatibility](https://gvisor.dev/docs/user_guide/compatibility/)
- [gVisor Networking Security Blog](https://gvisor.dev/blog/2020/04/02/gvisor-networking-security/)
- [Unikraft Compatibility Concepts](https://unikraft.org/docs/concepts/compatibility)
- [Unikraft Binary Compatibility Guide](https://unikraft.org/guides/bincompat)
- [Unikraft Syscall Shim (ASPLOS'22 Tutorial)](https://asplos22.unikraft.org/syscall_shim-bincompat/)
- [Linux Kernel Subsystem Documentation](https://docs.kernel.org/subsystem-apis.html)
- [Linux syscalls(2) Manual Page](https://man7.org/linux/man-pages/man2/syscalls.2.html)
- [io_uring Overview (Lord of the io_uring)](https://unixism.net/loti/what_is_io_uring.html)
- [epoll (Wikipedia)](https://en.wikipedia.org/wiki/Epoll)
- [ext4 General Information (Linux Kernel Documentation)](https://docs.kernel.org/admin-guide/ext4.html)
