# Custom Syscall Migration Plan

This document analyzes the remaining custom syscalls in SentientOS and describes how each maps to Linux equivalents. Custom syscalls are identified by bit 63 being set in the syscall number. The goal is to replace them all with standard Linux syscalls so userspace programs use only musl libc / POSIX interfaces.

## Summary

| # | Custom Syscall | Linux Equivalent | Callers | Difficulty |
|---|----------------|-----------------|---------|------------|
| ~~3~~ | ~~`sys_execute`~~ | ~~`clone` + `execve`~~ | ~~`init.rs`, `sesh.rs`, `stress.rs`~~ | ~~Hard~~ **DONE** |
| ~~4~~ | ~~`sys_wait`~~ | ~~`wait4` (NR 260)~~ | ~~`init.rs`, `sesh.rs`, `stress.rs`~~ | ~~Medium~~ **DONE** |
| ~~6~~ | ~~`sys_open_udp_socket`~~ | ~~`socket` + `bind`~~ | ~~`udp.rs`~~ | ~~Hard~~ **DONE** |
| ~~7~~ | ~~`sys_write_back_udp_socket`~~ | ~~`sendto`~~ | ~~`udp.rs`~~ | ~~Hard~~ **DONE** |
| ~~8~~ | ~~`sys_read_udp_socket`~~ | ~~`recvfrom`~~ | ~~`udp.rs`~~ | ~~Hard~~ **DONE** |

## Detailed Analysis

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

### 6. `sys_open_udp_socket` — `socket` + `bind` (**DONE**)

Migrated to `socket(AF_INET, SOCK_DGRAM, 0)` + `bind(fd, sockaddr_in, addrlen)`. Userspace uses `std::net::UdpSocket::bind()`.

---

### 7. `sys_write_back_udp_socket` — `sendto` (**DONE**)

Migrated to `sendto(fd, buf, len, flags, dest_addr, addrlen)`. Now supports arbitrary destinations (not just reply-to-sender). Userspace uses `std::net::UdpSocket::send_to()`.

---

### 8. `sys_read_udp_socket` — `recvfrom` (**DONE**)

Migrated to `recvfrom(fd, buf, len, flags, src_addr, addrlen)`. Per-datagram buffering preserves sender address for each datagram. Supports `O_NONBLOCK` via `ioctl(FIONBIO)`. Userspace uses `std::net::UdpSocket::recv_from()`.

---

## Migration Order

Recommended order based on dependencies and complexity:

1. **`sys_execute` + `sys_wait`** (#3, #4) — Implement `clone`+`execve`+`wait4`. Must be done together since wait depends on parent-child relationships established by clone.
2. **Socket syscalls** (#6, #7, #8) — Implement `socket`+`bind`+`recvfrom`+`sendto`. Must be done together.

## Key Design Decisions

### Process creation: clone + execve vs. posix_spawn
The current `sys_execute` is atomic (one syscall creates and starts a process). Linux splits this into clone (fork the process) and execve (replace the image). The simplest path is `CLONE_VFORK | CLONE_VM` semantics where the parent suspends until the child calls `execve`, avoiding the need for copy-on-write page tables. This matches how musl's `posix_spawn` works internally.

### Non-blocking I/O
The `udp.rs` program polls both stdin and a UDP socket in a tight loop. After migration, this pattern should use `ppoll` with both an stdin fd and a socket fd, or use non-blocking fds with `O_NONBLOCK`. Enhancing `ppoll` to support `POLLIN` on real fds is the cleanest path.

