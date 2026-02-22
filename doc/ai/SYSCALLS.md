# Syscall Handling

## Overview

SentientOS uses Linux-compatible syscalls exclusively. All syscall handlers are async, enabling blocking operations (e.g. nanosleep, read) without blocking the kernel.

## Syscall Dispatch

**File:** `kernel/src/interrupts/trap.rs`

```rust
fn handle_syscall() {
    let trap_frame = Cpu::read_trap_frame();
    let task = Task::new(async { handler.handle(&trap_frame).await });
    if let Poll::Ready(result) = task.poll(&mut cx) {
        trap_frame[Register::a0] = result;
        sepc += 4;  // Skip ecall
    } else {
        thread.set_syscall_task_and_suspend(task);
        scheduler.schedule();
    }
}
```

## Supported Syscalls

**File:** `kernel/src/syscalls/linux.rs`

| Syscall | Args | Description |
|---------|------|-------------|
| bind | fd, addr, addrlen | Bind socket to address/port |
| brk | brk | Adjust heap break |
| clock_nanosleep | clockid, flags, request, remain | Sleep with clock selection |
| clone | flags, stack, ptid, tls, ctid | Create child process (CLONE_VM\|CLONE_VFORK) |
| close | fd | Close file descriptor |
| execve | filename, argv, envp | Replace process image |
| exit_group | status | Exit process (stores exit status, then kills process) |
| fcntl | fd, cmd, arg | File descriptor control (F_GETFL/F_SETFL, O_NONBLOCK) |
| getppid | | Get parent process ID |
| gettid | | Get thread ID |
| ioctl | fd, op, arg | Device control (+ SentientOS extensions, FIONBIO for sockets) |
| mmap | addr, len, prot, flags, fd, off | Map memory |
| munmap | addr, len | Unmap memory |
| nanosleep | duration, rem | Sleep |
| ppoll | fds, n, timeout, mask | Poll file descriptors |
| prctl | | Process control |
| read | fd, buf, count | Read from fd |
| recvfrom | fd, buf, len, flags, src_addr, addrlen | Receive UDP datagram with sender address |
| rt_sigaction | sig, act, oact, size | Signal action |
| rt_sigprocmask | how, set, oldset, size | Signal mask |
| sendto | fd, buf, len, flags, dest_addr, addrlen | Send UDP datagram to destination |
| set_tid_address | tidptr | Set clear_child_tid |
| sigaltstack | uss, uoss | Signal stack |
| socket | domain, type, protocol | Create socket (AF_INET + SOCK_DGRAM only) |
| wait4 | pid, status, options, rusage | Wait for child process (supports WNOHANG) |
| write | fd, buf, count | Write to fd |
| writev | fd, iov, iovcnt | Vectored write |

### LinuxSyscallHandler

```rust
pub struct LinuxSyscallHandler {
    handler: SyscallHandler,
}

impl LinuxSyscalls for LinuxSyscallHandler {
    async fn read(&mut self, fd, buf, count) -> Result<isize, Errno>;
    async fn write(&mut self, fd, buf, count) -> Result<isize, Errno>;
    async fn exit_group(&mut self, status) -> Result<isize, Errno>;
    // ... other syscalls
}
```

### SyscallHandler

```rust
pub struct SyscallHandler {
    current_process: ProcessRef,
    current_thread: ThreadRef,
    current_tid: Tid,
}

impl SyscallHandler {
    pub fn new() -> Self;
    pub fn current_tid(&self) -> Tid;
    pub fn current_process(&self) -> &ProcessRef;
    pub fn current_thread(&self) -> &ThreadRef;
    pub fn sys_exit(&mut self, status: isize);
}
```

### SentientOS ioctl Extensions

Custom kernel functionality exposed via `ioctl` on stdout. Constants and userspace wrappers defined in `common/src/ioctl.rs`.

| Command | Value | Description |
|---------|-------|-------------|
| SENTIENT_PANIC | 0x5301 | Trigger kernel panic from userspace |
| SENTIENT_LIST_PROGRAMS | 0x5302 | Print list of available programs |

## Userspace Pointer Validation

**File:** `kernel/src/syscalls/linux_validator.rs`

```rust
pub struct LinuxUserspaceArg<P: LinuxPointer>(UserspacePtr<P>);

impl LinuxUserspaceArg<*const T> {
    pub fn validate_ptr(&self) -> Result<T, Errno>;
    pub fn validate_str(&self, len: usize) -> Result<&str, Errno>;
    pub fn validate_slice(&self, len: usize) -> Result<&[T], Errno>;
}

impl LinuxUserspaceArg<*mut T> {
    pub fn write(&self, value: T) -> Result<(), Errno>;
    pub fn write_slice(&self, data: &[T]) -> Result<(), Errno>;
    pub fn write_if_not_none(&self, value: T) -> Result<(), Errno>;
}
```

### Validation Process

1. Check pointer is in userspace address range
2. Translate virtual address through page tables
3. Verify page permissions (read/write)
4. Return kernel-accessible physical address

## Adding a New Syscall

1. Add to `linux_syscalls!` macro in `kernel/src/syscalls/linux.rs`:
```rust
linux_syscalls! {
    SYSCALL_NR_MYSYSCALL => mysyscall(arg1: type1, arg2: type2);
}
```

2. Implement handler in `LinuxSyscalls` impl:
```rust
async fn mysyscall(&mut self, arg1: LinuxUserspaceArg<type1>, arg2: LinuxUserspaceArg<type2>)
    -> Result<isize, Errno>
{
    let arg1 = arg1.validate_ptr()?;
    // Implementation
    Ok(0)
}
```

## Error Handling

Linux syscalls return:
- Success: positive value or 0
- Error: `-Errno` (negative errno value)

```rust
let ret = match result {
    Ok(ret) => ret,
    Err(errno) => -(errno as isize),
};
trap_frame[Register::a0] = ret as usize;
```

## Key Files

| File | Purpose |
|------|---------|
| kernel/src/syscalls/mod.rs | Module exports |
| kernel/src/syscalls/linux.rs | Linux syscall implementations |
| kernel/src/syscalls/handler.rs | SyscallHandler |
| kernel/src/syscalls/macros.rs | linux_syscalls! macro |
| kernel/src/syscalls/linux_validator.rs | LinuxUserspaceArg validation |
| common/src/ioctl.rs | SentientOS ioctl constants + userspace wrappers |
| headers/src/syscall_types.rs | Syscall type definitions |
| headers/src/errno.rs | Error codes |
