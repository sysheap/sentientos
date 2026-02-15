# Custom Syscall Migration Plan

This document analyzes all 11 custom syscalls in SentientOS and describes how each maps to Linux equivalents. Custom syscalls are identified by bit 63 being set in the syscall number. The goal is to replace them all with standard Linux syscalls so userspace programs use only musl libc / POSIX interfaces.

## Summary

| # | Custom Syscall | Linux Equivalent | Callers | Difficulty |
|---|----------------|-----------------|---------|------------|
| 0 | `sys_write` | `write` (NR 64) | None | **MIGRATED** — deleted |
| 1 | `sys_read_input` | `read` (NR 63) with non-blocking stdin | `udp.rs` | Medium |
| 2 | `sys_exit` | `exit_group` (NR 94) | `sesh.rs` | Trivial |
| 3 | `sys_execute` | `clone` + `execve` | `init.rs`, `sesh.rs`, `stress.rs` | Hard |
| 4 | `sys_wait` | `wait4` (NR 260) | `init.rs`, `sesh.rs`, `stress.rs` | Medium |
| 5 | `sys_mmap_pages` | `mmap` (NR 222) | None | **MIGRATED** — deleted |
| 6 | `sys_open_udp_socket` | `socket` + `bind` | `udp.rs` | Hard |
| 7 | `sys_write_back_udp_socket` | `sendto` | `udp.rs` | Hard |
| 8 | `sys_read_udp_socket` | `recvfrom` | `udp.rs` | Hard |
| 9 | `sys_panic` | `kill(SIGABRT)` or null-ptr write | `panic.rs` | Easy |
| 10 | `sys_print_programs` | `ioctl` or read from pseudo-file | `sesh.rs` | Medium |

## Detailed Analysis

### 0. `sys_write` — DELETE

**Current behavior:** Validates a string pointer from userspace, prints it via `print!("{s}")`.

**Linux equivalent:** Already implemented — `write` (NR 64) and `writev` (NR 66) exist in `linux.rs` and handle fd 1 (stdout) and fd 2 (stderr).

**Callers:** None. All userspace programs use `println!` which goes through musl libc → Linux `writev`/`write`.

**Migration:** Delete the syscall. No userspace changes needed.

---

### 1. `sys_read_input` — `read` with non-blocking stdin

**Current behavior:** Pops one byte from the kernel's `STDIN_BUFFER` if available, returns `None` if empty. Non-blocking.

**Caller:** `udp.rs` — uses it in a busy-loop to poll for keyboard input alongside UDP socket data.

**Linux equivalent:** The Linux `read` syscall (NR 63) already exists in the kernel but is **blocking** — it uses `ReadStdin::new(count).await` which suspends the thread until data is available.

**Migration steps:**
1. The `udp.rs` program needs non-blocking stdin. Options:
   - Support `O_NONBLOCK` on fd 0 via `fcntl`, making `read` return `EAGAIN` when no data is available.
   - Support `ppoll` with a timeout on fd 0, so the program can poll stdin with a zero timeout.
2. Rewrite `udp.rs` to use `read(0, &mut buf, 1)` instead of `sys_read_input()`. If non-blocking, handle `EAGAIN`. If using `ppoll`, check readiness before reading.
3. The current `ppoll` implementation asserts `fd.events == 0` — it needs to support `POLLIN` on fd 0.

**Complexity:** Medium. Requires enhancing the kernel's `ppoll` or `read` to support non-blocking I/O on stdin.

---

### 2. `sys_exit` — `exit_group`

**Current behavior:** Sets `process_exit = true`, calls `scheduler.kill_current_process()`, logs exit status.

**Caller:** `sesh.rs` — calls `sys_exit(0)` when user types "exit" or "q".

**Linux equivalent:** `exit_group` (NR 94) already exists and internally delegates to `self.handler.sys_exit(...)`. It is the standard way musl libc exits processes.

**Migration:** Replace `sys_exit(0)` in `sesh.rs` with `std::process::exit(0)`, which calls `exit_group` via musl. No kernel changes needed.

---

### 3. `sys_execute` — `clone` + `execve`

**Current behavior:** Takes a program name and arguments. Calls `scheduler.start_program(name, &args)` which creates a new process from an embedded ELF binary, adds it to the scheduler, and returns a `Tid`. The operation is atomic — the new process is fully created in the parent's syscall handler.

**Callers:**
- `init.rs` — starts the shell: `sys_execute("sesh", &[])`
- `sesh.rs` — runs user commands: `sys_execute(prog_name, &args)`
- `stress.rs` — spawns load-test processes: `sys_execute("loop", &[])`

**Linux equivalent:** `clone` (NR 220) + `execve` (NR 221). On Linux, process creation is a two-step fork+exec:
1. `clone(flags)` — duplicates the current process
2. `execve(path, argv, envp)` — replaces the process image

**Migration steps:**
1. Implement `clone` syscall. Since programs are embedded ELFs (not files on disk), the semantics differ from Linux. A reasonable approach:
   - Use `CLONE_VFORK | CLONE_VM` semantics: the parent blocks until the child calls `execve`, and both share the address space until that point. This avoids implementing full copy-on-write page tables.
   - Alternatively, implement a minimal `fork` that copies the address space.
2. Implement `execve` syscall. Since there's no filesystem, the `path` argument would be the program name looked up in the embedded `PROGRAMS` table.
3. Update userspace to use `fork()`/`exec()` from musl libc, or implement a `posix_spawn`-style wrapper.

**Complexity:** Hard. This is the most complex migration. Requires implementing clone/fork semantics in the kernel.

---

### 4. `sys_wait` — `wait4`

**Current behavior:** Takes a `Tid` and blocks the current thread until that thread exits. Returns `Err(InvalidPid)` if the target doesn't exist.

**Callers:**
- `init.rs` — waits for the shell
- `sesh.rs` — waits for foreground processes
- `stress.rs` — waits for all spawned processes

**Linux equivalent:** `wait4` (NR 260) or `waitpid`. Waits for a child process to change state and returns its exit status.

**Migration steps:**
1. Implement `wait4(pid, &status, options, &rusage)` syscall.
2. Key difference: Linux `wait4` waits for **child** processes, not arbitrary thread IDs. The kernel needs to track parent-child relationships (which `clone`/`fork` will establish).
3. Support `WNOHANG` flag for non-blocking wait.
4. Userspace switches to `waitpid()` from musl libc.

**Complexity:** Medium. Straightforward once clone/fork establishes parent-child relationships.

---

### 5. `sys_mmap_pages` — DELETE

**Current behavior:** Allocates N pages and maps them read-write into the current process's address space. Returns the virtual address pointer.

**Linux equivalent:** Already implemented — `mmap` (NR 222) in `linux.rs` supports `MAP_ANONYMOUS | MAP_PRIVATE` with various protection modes.

**Callers:** None. All userspace programs use musl libc's `mmap` which calls the Linux `mmap` syscall.

**Migration:** Delete the syscall. No userspace changes needed.

---

### 6. `sys_open_udp_socket` — `socket` + `bind`

**Current behavior:** Takes a port number, acquires a socket from the global socket table, attaches it to the current process, and returns a `UDPDescriptor` (process-local handle).

**Caller:** `udp.rs` via the `UdpSocket::try_open(port)` wrapper.

**Linux equivalent:**
- `socket(AF_INET, SOCK_DGRAM, 0)` — creates a UDP socket, returns an fd
- `bind(fd, &sockaddr_in, len)` — binds to a port

**Migration steps:**
1. Implement a per-process file descriptor table in the kernel. Currently, processes don't have an fd table (only hard-coded 0/1/2 for stdin/stdout/stderr).
2. Implement `socket` syscall (NR 198) that creates a socket and returns an fd.
3. Implement `bind` syscall (NR 200) that binds the socket to a port.
4. Rewrite `udp.rs` to use `socket()` and `bind()` from musl libc.

**Complexity:** Hard. Requires implementing the fd table and socket-to-fd integration.

---

### 7. `sys_write_back_udp_socket` — `sendto`

**Current behavior:** Takes a `UDPDescriptor` and a byte buffer. Looks up the source IP/port from the last received packet on that socket, resolves the destination MAC from the ARP cache, constructs a UDP packet, and sends it. Only supports "reply to sender" — not arbitrary destinations.

**Caller:** `udp.rs` via `socket.transmit(data)`.

**Linux equivalent:** `sendto(fd, buf, len, flags, &dest_addr, addrlen)` (NR 206).

**Migration steps:**
1. Implement `sendto` syscall that takes a destination address (unlike the current reply-only semantics).
2. The kernel network stack needs to accept an explicit destination IP/port rather than inferring from the last received packet.
3. Rewrite `udp.rs` to use `sendto()` from musl libc with the destination address.

**Complexity:** Hard. Coupled with the fd table work from `sys_open_udp_socket`.

---

### 8. `sys_read_udp_socket` — `recvfrom`

**Current behavior:** Takes a `UDPDescriptor` and a mutable buffer. Calls `receive_and_process_packets()` first (processes any pending NIC packets), then reads available data from the socket buffer. Non-blocking — returns 0 if no data.

**Caller:** `udp.rs` via `socket.receive(&mut buffer)`.

**Linux equivalent:** `recvfrom(fd, buf, len, flags, &src_addr, &addrlen)` (NR 207).

**Migration steps:**
1. Implement `recvfrom` syscall that reads from a socket fd and provides the sender's address.
2. Decide on blocking vs. non-blocking behavior. The current code is non-blocking (returns 0 immediately). Standard `recvfrom` is blocking unless `MSG_DONTWAIT` or `O_NONBLOCK` is set.
3. Packet processing (`receive_and_process_packets`) should happen asynchronously (interrupt-driven or in a kernel thread) rather than synchronously during the syscall.

**Complexity:** Hard. Coupled with the fd table and socket work.

---

### 9. `sys_panic` — `kill(SIGABRT)` or crash

**Current behavior:** Triggers `panic!("Userspace triggered kernel panic")` — crashes the entire kernel.

**Caller:** `panic.rs` — a test program that intentionally triggers a kernel panic.

**Linux equivalent:** There's no Linux syscall that intentionally panics the kernel. Options:
- `kill(getpid(), SIGABRT)` — terminates the process with a signal (not the kernel)
- Write to a null pointer / execute an illegal instruction — triggers a fault
- For testing purposes, a custom ioctl or debug-only mechanism could remain

**Migration steps:**
1. If the intent is "crash the kernel for testing," this could remain as a custom debug mechanism or be triggered via a magic write to a debug device.
2. If the intent is "abort the process," replace with `std::process::abort()` which raises SIGABRT.
3. Since this is only used in a test program, the simplest approach is to write to a null pointer (which the kernel's trap handler can detect and panic on if desired).

**Complexity:** Easy. The test program can use any crash mechanism.

---

### 10. `sys_print_programs` — `ioctl` or pseudo-file

**Current behavior:** Iterates the `PROGRAMS` array (embedded ELF binaries) and prints each name to stdout. The kernel directly produces output on behalf of the process.

**Caller:** `sesh.rs` — the shell's `help` command prints available programs.

**Linux equivalent:** No direct Linux equivalent. The kernel needs to expose the program list to userspace. Options:

**Option A: `ioctl` on stdout**
Define a custom `ioctl` number (e.g., `TIOC_LIST_PROGRAMS`). Userspace calls `ioctl(1, TIOC_LIST_PROGRAMS, &buffer)` to read the program list into a buffer.

**Option B: Pseudo-file (e.g., `/proc/programs`)**
Expose a virtual file that lists programs. Userspace reads it with `open` + `read`. Requires implementing `open` syscall and a minimal virtual filesystem, which is substantial.

**Option C: Dedicated simple syscall**
Keep a Linux-numbered syscall that fills a userspace buffer with the program list. Pragmatic but not standard.

**Recommended approach:** Option A (`ioctl`) is the least invasive. The kernel already has an `ioctl` handler in `linux.rs`. Extend it with a custom operation code that writes the program list to a userspace buffer.

**Migration steps:**
1. Define a custom ioctl number for listing programs.
2. Extend the `ioctl` handler to support it — serialize program names into a userspace buffer.
3. Update `sesh.rs` to call `ioctl(1, TIOC_LIST_PROGRAMS, buf.as_mut_ptr())` and print the result.

**Complexity:** Medium.

---

## Migration Order

Recommended order based on dependencies and complexity:

1. **Delete unused syscalls** — `sys_write` (#0) and `sys_mmap_pages` (#5). Zero risk.
2. **`sys_exit`** (#2) — One-line change in `sesh.rs` to use `std::process::exit`.
3. **`sys_panic`** (#9) — Replace with null-ptr write or `abort()` in `panic.rs`.
4. **`sys_print_programs`** (#10) — Add custom ioctl, update shell.
5. **`sys_read_input`** (#1) — Enhance `ppoll`/`read` for non-blocking stdin, update `udp.rs`.
6. **`sys_execute` + `sys_wait`** (#3, #4) — Implement `clone`+`execve`+`wait4`. Must be done together since wait depends on parent-child relationships established by clone.
7. **Socket syscalls** (#6, #7, #8) — Implement fd table, then `socket`+`bind`+`recvfrom`+`sendto`. Must be done together.

## Key Design Decisions

### Process creation: clone + execve vs. posix_spawn
The current `sys_execute` is atomic (one syscall creates and starts a process). Linux splits this into clone (fork the process) and execve (replace the image). The simplest path is `CLONE_VFORK | CLONE_VM` semantics where the parent suspends until the child calls `execve`, avoiding the need for copy-on-write page tables. This matches how musl's `posix_spawn` works internally.

### File descriptor table
The kernel currently has no fd table — stdin/stdout/stderr are hard-coded checks on fd values 0/1/2. Migrating socket syscalls requires a proper fd table in each process. This is also a prerequisite for future features like pipes, file I/O, and dup2.

### Non-blocking I/O
The `udp.rs` program polls both stdin and a UDP socket in a tight loop. After migration, this pattern should use `ppoll` with both an stdin fd and a socket fd, or use non-blocking fds with `O_NONBLOCK`. Enhancing `ppoll` to support `POLLIN` on real fds is the cleanest path.

### Program list exposure
Since there's no filesystem, `ioctl` is the pragmatic choice for exposing the embedded program list. A pseudo-filesystem (`/proc`) would be more Linux-like but requires substantially more infrastructure.
