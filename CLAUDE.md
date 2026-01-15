# SentientOS - AI Agent Reference

RISC-V 64-bit hobby OS kernel written in Rust. No third-party runtime dependencies.

## Quick Commands

```bash
just run          # Build and run in QEMU
just test         # Run unit tests + system tests
just ci           # Run all CI checks (clippy, fmt, tests, miri)
just build        # Build kernel with userspace
just system-test  # Run only system tests
just unit-test    # Run only unit tests
just clippy       # Run linter
just disassm      # Disassemble kernel
just addr2line 0x1234  # Get source line for kernel address
```

## Project Structure

```
kernel/           # Main kernel (RISC-V 64-bit, no_std)
userspace/        # Userspace programs (musl libc)
common/           # Shared no_std library
system-tests/     # Integration tests (run on x86, test via QEMU)
headers/          # Linux C header bindings via bindgen
doc/ai/           # Detailed AI documentation (see OVERVIEW.md)
```

## Key Kernel Subsystems

| Directory | Purpose |
|-----------|---------|
| kernel/src/memory/ | Page allocator, page tables, heap |
| kernel/src/processes/ | Process, thread, scheduler |
| kernel/src/syscalls/ | syscall handlers |
| kernel/src/interrupts/ | Trap handling, PLIC, timer |
| kernel/src/net/ | UDP network stack |
| kernel/src/drivers/virtio/ | VirtIO network driver |
| kernel/src/io/ | UART, stdin buffer |

## Debugging

### Logging Macros
- `info!()` - Always printed. Use sparingly (clutters user output).
- `debug!()` - Conditional. Enable per-module. Leave in code.
- `warn!()` - Always printed.

### Enable Debug Output for a Module
Edit `kernel/src/logging/configuration.rs`:
```rust
// Add to LOG_FOLLOWING_MODULES to enable:
const LOG_FOLLOWING_MODULES: &[&str] = &["kernel::processes::scheduler"];

// Or remove from DONT_LOG_FOLLOWING_MODULES if blocked there
```

### GDB Debugging
```bash
just debug        # Start QEMU + GDB in tmux
just debugf FUNC  # Debug with breakpoint on function
```

## Testing Strategy

### System Tests (Preferred for AI iteration)
Located in `system-tests/src/tests/`. Run the OS in QEMU and interact via stdin/stdout.

```bash
# Run all system tests
just system-test

# Run specific test
cargo nextest run --release --manifest-path system-tests/Cargo.toml \
    --target x86_64-unknown-linux-gnu test_name
```

### Writing Throw-Away Tests
Add to `system-tests/src/tests/basics.rs` or create new test file:
```rust
#[tokio::test]
async fn my_test() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;
    let output = sentientos.run_prog("prog1").await?;
    assert_eq!(output, "expected");
    Ok(())
}
```

### Unit Tests
Kernel unit tests use `#[test_case]` macro (custom test framework).

## Adding Userspace Programs

1. Create `userspace/src/bin/myprogram.rs`
2. Run `just build` (automatically embedded in kernel)
3. Available in shell as `myprogram`

## Key Files Quick Reference

| Purpose | File |
|---------|------|
| Kernel entry | kernel/src/main.rs |
| Syscall dispatch | kernel/src/syscalls/handler.rs |
| Process struct | kernel/src/processes/process.rs |
| Scheduler | kernel/src/processes/scheduler.rs |
| Page tables | kernel/src/memory/page_tables.rs |
| Trap handler | kernel/src/interrupts/trap.rs |
| System test infra | system-tests/src/infra/qemu.rs |
| Log config | kernel/src/logging/configuration.rs |

## Detailed Documentation

See `doc/ai/OVERVIEW.md` for comprehensive subsystem documentation including:
- Per-CPU struct architecture (`kernel/src/cpu.rs`) for multi-core support
- Async syscall model
- Memory layout and page tables

## Development Guidelines

**Prefer less code.** Achieve the same result with fewer lines. Avoid unnecessary abstractions, helpers for one-time operations, or premature optimization. Simplify existing code when touching it for a feature.

**Fail fast with assertions.** Use `assert!` instead of `debug_assert!`. An inconsistent state in the kernel should panic immediately rather than continue with corrupted data. Crashing early makes bugs easier to diagnose and prevents cascading failures.

**No bloated comments.** Add comments only when explaining invariants or non-obvious logic. Never add comments that restate what the code does, separators, or decorative formatting.

**Commit automatically.** After completing a task, commit without waiting for user intervention. Before committing:
- Run `just clippy` to ensure no warnings
- Remove any dead or unused code introduced by your changes

**Commit incrementally.** Commit each small working step toward a larger goal. Include test code in commits. This enables incremental progress verification rather than large, hard-to-debug changesets.

**Keep docs in sync.** Update `CLAUDE.md` and `doc/ai/*` when discovering inconsistencies or implementing new features.

**Network port conflict.** Only one QEMU instance with `--net` can run at a time (port 1234). See `doc/ai/DEBUGGING.md` for all QEMU wrapper options.
