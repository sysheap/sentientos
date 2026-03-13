# Solaya Linux Compatibility: Master Plan

This is the synthesis of 5 detailed planning documents. Read this first for the big picture, then dive into individual documents for details.

## Vision

Rewrite the Linux kernel in Rust for memory safety, better abstractions, and cleaner code. The result is a 100% binary-compatible Linux clone that runs unmodified Linux userspace programs. Only the kernel is rewritten -- all userspace comes from existing projects (musl, dash, coreutils, etc.).

## Document Organization

- **`doc/ai/`** -- Documents the **current** system. Updated alongside code changes. The source of truth for what exists today. See `doc/ai/OVERVIEW.md` for the index.
- **`plans/`** -- Documents the **future** direction. Updated when strategy changes. Describes what to build and in what order.

Plans reference `doc/ai/` for current state rather than duplicating it. This avoids stale snapshots.

## Detailed Plans

| Document | Contents |
|----------|----------|
| [01-roadmap.md](01-roadmap.md) | Phase-by-phase roadmap, syscall coverage plan, architectural decisions, risk assessment |
| [02-testing-strategy.md](02-testing-strategy.md) | Test suite integration (LTP, libc-test, kselftest), compliance tracking, CI tiers |
| [03-driver-framework.md](03-driver-framework.md) | Linux driver model design, VirtIO focus, sysfs, x86 considerations |
| [04-build-system.md](04-build-system.md) | Build system evaluation, multi-arch support, CI/CD pipeline |
| [05-organization.md](05-organization.md) | Work tracking, AI agent workflow, parallel agents, learning from reviews |

## What to Do First

The roadmap and all supporting plans converge on a clear starting sequence:

### Immediate (Next 2-4 Weeks)

1. **Integrate libc-test** (02-testing-strategy.md, Phase 1)
   - Cross-compile musl's test suite for riscv64-musl
   - Package into ext2 disk image, load via VirtIO block
   - Write a compliance-runner program
   - Set up CI compliance job
   - This gives an objective "how far are we" metric immediately

2. **Implement CoW fork** (01-roadmap.md, Phase 1.1)
   - Single biggest performance bottleneck
   - Every fork+exec copies megabytes that get immediately discarded
   - Foundation for everything that follows

3. **Add missing critical syscalls** (01-roadmap.md, Phase 1.6)
   - `uname`, `pread64/pwrite64`, `getrandom`, `truncate/ftruncate`
   - `renameat`, `linkat/symlinkat`, `fchmod/fchmodat`, `fchown/fchownat`
   - These unblock running more real programs

### Short-Term (1-3 Months)

4. **Integrate LTP syscall tests** (02-testing-strategy.md, Phase 2)
   - Start with ~100 "smoke" tests covering implemented syscalls
   - Set up the ratchet: once a test passes, it must never regress
   - Failing tests drive the implementation backlog

5. **Refactor VirtIO initialization** (03-driver-framework.md, Phase 1)
   - Eliminate ~200 lines of duplicated boilerplate across 4 drivers
   - Create `VirtioDevice::from_pci()` shared initializer
   - Replace hardcoded PLIC interrupt dispatch with handler table

6. **Demand paging and lazy allocation** (01-roadmap.md, Phase 1.2)
   - mmap currently allocates all pages eagerly
   - Required for running larger programs

7. **Robust VFS layer** (01-roadmap.md, Phase 1.4)
   - Symlinks, rename, proper O_APPEND/O_EXCL handling
   - File metadata (uid/gid/mode/timestamps)

### Medium-Term (3-6 Months)

8. **epoll** (01-roadmap.md, Phase 2.2)
   - Required by every modern event loop (nginx, Redis, Node.js, Go, tokio)
   - Design the readiness notification trait early in the VFS layer

9. **Pseudo-terminals** (01-roadmap.md, Phase 2.3)
   - Required for SSH, screen/tmux, and any pty-based program

10. **Unix domain sockets** (01-roadmap.md, Phase 2.4)
    - Required by X11, DBus, Docker, most language runtimes

## Key Architectural Decisions

| Decision | Recommendation | Rationale |
|----------|---------------|-----------|
| Async syscall model | Keep it | Sound, scales well, similar to io_uring conceptually |
| Driver reuse | Write native Rust, don't reuse Linux C | Compatibility surface too large; VirtIO drivers are small |
| Driver model | Linux-like bus/device/driver traits | sysfs paths must look correct to Linux userspace |
| VirtIO only | Yes, for now | QEMU target; validated by gVisor/Firecracker |
| Build system | Keep Nix + just, migrate to xtask when adding x86 | Best short-term option; Hermit OS proves xtask works |
| x86_64 priority | Defer until kernel is more mature | Most work is arch-independent; KVM speedup is nice-to-have |
| Test strategy | libc-test first, then LTP | libc-test is small and tests the exact musl/kernel interface |
| Project tracking | GitHub Issues + task spec files for complex work | Agents need rich context; simple tasks stay as issues |
| Parallel agents | 2-3 max, partitioned by subsystem | Review bandwidth is the bottleneck |
| Progress metric | LTP pass rate + capability milestones | Not raw syscall count |

## Milestones

| # | Target | Key Indicator |
|---|--------|---------------|
| M1 | CoW fork + demand paging | Fork is 10x faster |
| M2 | Run BusyBox unmodified | busybox ls, grep, find work |
| M3 | Run bash | bash starts and runs scripts |
| M4 | epoll works | Simple event loop program works |
| M5 | Run Python interpreter | `python3 -c "print('hello')"` works |
| M6 | Run Redis | redis-server starts, redis-cli GET/SET works |
| M7 | Run nginx | Serves static files over HTTP |
| M8 | Container basics | unshare + chroot + basic namespaces |
| M9 | x86_64 boot | Kernel boots on QEMU x86_64 |
| M10 | Self-hosting | Compile a Rust program on Solaya |

## Risk Assessment (Hardest Parts)

1. **TCP/IP stack** (Very High) -- Linux's TCP is 30K lines accumulated over 30 years. Must be written from scratch (no third-party runtime deps).
2. **Filesystem complexity** (High) -- Writable ext2/ext4 needs crash consistency. Use tmpfs + virtio-fs/9p as stopgaps.
3. **epoll** (Medium-High) -- Architecturally pervasive; every FD type needs readiness notification.
4. **Namespaces/cgroups** (High) -- Touch every subsystem. Implement incrementally.
5. **Compatibility edge cases** (Ongoing) -- 40% of effort goes here. Programs depend on exact errno values and flag behavior.

## What NOT to Do

- Don't build custom orchestration tooling for AI agents. The maintainer is the orchestrator.
- Don't rewrite the build system before adding x86_64.
- Don't try to reuse Linux C drivers via compatibility shims.
- Don't implement namespaces or cgroups before the basics are solid.
- Don't track progress by syscall count alone.
- Don't run more than 3 parallel agents.
- Don't pursue syzkaller until >50% LTP compliance.
