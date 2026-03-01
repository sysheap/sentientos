# Futex: FUTEX_WAIT returns EINVAL instead of 0

## The bug

`futex(FUTEX_WAIT)` returns `-22 (EINVAL)` when the value at the address doesn't match the expected value. Linux returns `0` (or `-EAGAIN` depending on version). musl expects `0` or `-EAGAIN` and retries on either, but `-EINVAL` causes it to spin-loop rapidly on the futex syscall.

## How it was discovered

When rewriting `udp.rs` to use two threads with `Arc<Mutex<Option<SocketAddr>>>`, the spawned thread entered a tight spin-loop of `futex(FUTEX_WAIT)` calls all returning `EINVAL`. Syscall trace:

```
[SYSCALL ENTER] tid=4 futex(uaddr: 0x79870, op: 128, val: 0xffffffff80000002, ...)
[SYSCALL EXIT]  tid=4 futex = -22 (EINVAL)
[SYSCALL ENTER] tid=4 futex(uaddr: 0x79870, op: 128, val: 0xffffffff80000002, ...)
[SYSCALL EXIT]  tid=4 futex = -22 (EINVAL)
... (hundreds of times)
```

The workaround in `udp.rs` was to replace `Mutex` with `AtomicU64` (lock-free), avoiding futex entirely.

## Where to look

- `kernel/src/processes/futex.rs` — `FutexWait::poll()` returns `Poll::Ready(0)` when value doesn't match (line 48), which should produce `Ok(0)` at the syscall layer
- `kernel/src/syscalls/linux.rs:565-588` — futex syscall handler wraps the result as `Ok(result as isize)`
- The code looks correct on paper: `FutexWait` returns 0, handler returns `Ok(0)`. Yet userspace sees -22.

## Likely causes

1. **Syscall dispatch macro issue** — something between `Ok(0)` in the handler and the actual register write to userspace transforms the value. Check how the `linux_syscalls!` macro maps `Result<isize, Errno>` back to the a0 register.

2. **`val` parameter truncation** — the trace shows `val: 0xffffffff80000002` (64-bit). The handler declares `val: c_uint` (u32), so it should truncate to `0x80000002`. If the comparison in `FutexWait::poll()` reads a different bit width from memory, the values might never match, causing unexpected behavior.

3. **Tracer misreporting** — less likely, but verify the tracer in `kernel/src/syscalls/tracer.rs` isn't mangling the return value.

## Debugging approach

1. Add `debug!("futex_wait result: {}", result)` inside the `FUTEX_WAIT` branch before `Ok(result as isize)` to confirm the kernel-side return value
2. If it's 0 there, the bug is in the syscall return path (macro/register write)
3. If it's not 0, trace deeper into `FutexWait::poll()`

## Impact

Any userspace code using `std::sync::Mutex`, `Condvar`, or other futex-backed primitives across threads will hit this spin-loop. It wastes CPU but eventually works because musl retries. Programs that heavily contend on mutexes will be noticeably slow.
