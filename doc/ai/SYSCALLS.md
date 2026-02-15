# Syscall Handling

## Overview

SentientOS provides two syscall interfaces:
1. **Linux syscalls** - Async handlers for musl libc compatibility
2. **Kernel syscalls** - Custom syscalls (bit 63 set in syscall number)

Always use Linux syscalls for new development. Kernel syscalls are obsolete and will be remove in the future.

Also linux syscall got support for async code (currently nanosleep and read). This makes writing blocking code way easier.

## Syscall Dispatch

**File:** `kernel/src/interrupts/trap.rs:64`

```rust
fn handle_syscall() {
    let nr = trap_frame[Register::a7];  // Syscall number

    if (1 << 63) & nr > 0 {
        // Fast syscall - synchronous, immediate return
        let ret = syscalls::handle_syscall(nr, arg, ret);
        trap_frame[Register::a0] = ret;
        sepc += 4;
    } else {
        // Linux syscall - async
        let task = Task::new(async { handler.handle(&trap_frame).await });
        // Poll and either return or suspend thread
    }
}
```

## Linux Syscalls

**File:** `kernel/src/syscalls/linux.rs`

### Supported Syscalls

| Syscall | Args | Description |
|---------|------|-------------|
| brk | brk | Adjust heap break |
| close | fd | Close file descriptor |
| exit_group | status | Exit process |
| gettid | | Get thread ID |
| ioctl | fd, op | Device control |
| mmap | addr, len, prot, flags, fd, off | Map memory |
| munmap | addr, len | Unmap memory |
| nanosleep | duration, rem | Sleep |
| ppoll | fds, n, timeout, mask | Poll file descriptors |
| prctl | | Process control |
| read | fd, buf, count | Read from fd |
| rt_sigaction | sig, act, oact, size | Signal action |
| rt_sigprocmask | how, set, oldset, size | Signal mask |
| set_tid_address | tidptr | Set clear_child_tid |
| sigaltstack | uss, uoss | Signal stack |
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

### Async Read Example

```rust
async fn read(&mut self, fd: c_int, buf: LinuxUserspaceArg<*mut u8>, count: usize)
    -> Result<isize, Errno>
{
    if fd != 0 { return Err(Errno::EBADF); }

    let data = ReadStdin::new(count).await;  // Async wait for input
    buf.write_slice(&data)?;
    Ok(data.len() as isize)
}
```

### mmap Implementation

```rust
async fn mmap(&mut self, addr: usize, length: usize, prot: c_uint,
              flags: c_uint, fd: c_int, offset: isize) -> Result<isize, Errno>
{
    // Convert prot to XWRMode
    let permission = match (prot & PROT_READ, prot & PROT_WRITE, prot & PROT_EXEC) {
        // ...
    };

    // Allocate pages
    if flags & MAP_FIXED != 0 {
        process.mmap_pages_with_address(num_pages, addr, permission)
    } else {
        process.mmap_pages(num_pages, permission)
    }
}
```

## Kernel Syscalls (Fast Path)

**File:** `kernel/src/syscalls/handler.rs`

Custom syscalls with bit 63 set for synchronous execution:

```rust
impl KernelSyscalls for SyscallHandler {
    fn sys_print_programs(&mut self);
    fn sys_panic(&mut self);
    fn sys_read_input(&mut self) -> Option<u8>;
    fn sys_execute(&mut self, name, args) -> Result<Tid, SysExecuteError>;
    fn sys_wait(&mut self, tid) -> Result<Tid, SysWaitError>;
    fn sys_open_udp_socket(&mut self, port) -> Result<UDPDescriptor, SysSocketError>;
    fn sys_write_back_udp_socket(&mut self, desc, buf);
    fn sys_read_udp_socket(&mut self, desc, buf);
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

## Userspace Pointer Validation

**File:** `kernel/src/syscalls/validator.rs`

All userspace pointers must be validated:

```rust
pub struct UserspaceArgument<T>(T);

impl<T: Pointer> UserspaceArgument<T> {
    pub fn validate(&self, handler: &SyscallHandler) -> Result<T, ValidationError>;
}
```

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

## Adding a New Linux Syscall

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

## Adding a New Kernel Syscall

1. Add to `KernelSyscalls` trait in `common/src/syscalls/kernel.rs`
2. Implement in `SyscallHandler` in `kernel/src/syscalls/handler.rs`
3. Add userspace wrapper in `userspace/src/lib.rs`

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
| kernel/src/syscalls/handler.rs | SyscallHandler, kernel syscalls |
| kernel/src/syscalls/macros.rs | linux_syscalls! macro |
| kernel/src/syscalls/validator.rs | UserspaceArgument validation |
| kernel/src/syscalls/linux_validator.rs | LinuxUserspaceArg validation |
| common/src/syscalls/kernel.rs | KernelSyscalls trait |
| headers/src/syscall_types.rs | Syscall type definitions |
| headers/src/errno.rs | Error codes |
