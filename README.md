# SentientOS

[![ci](https://github.com/sysheap/sentientos/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sysheap/sentientos/actions/workflows/ci.yml)

A RISC-V 64-bit hobby operating system kernel written in Rust. No third-party runtime dependencies — if third-party crates are used, they are build-time only.

Inspired by [SerenityOS](https://github.com/SerenityOS/serenity), this project exists because writing an OS from scratch is fun. Check out the [sysheap YouTube channel](http://www.youtube.com/@sysheap) for coding videos.

## Status

### Kernel

- **Memory management** — Bitmap page allocator with lazy zeroing, Sv39 page tables (3-level), kernel heap, per-process address spaces, mmap/munmap, brk
- **Processes & threads** — ELF loading, per-process file descriptor tables, thread states (Running/Runnable/Waiting/Zombie), async syscall model with wakers
- **Scheduler** — Per-CPU round-robin scheduler with 10ms quantum (50ms idle/powersave), global run queue
- **SMP** — Multi-core boot via SBI hart management, per-CPU state structs, inter-processor interrupts (IPI)
- **Syscalls** — 22+ Linux-compatible syscalls (read, write, mmap, nanosleep, wait4, ppoll, signal handling, etc.) plus custom kernel syscalls for program execution and UDP sockets
- **Interrupts** — RISC-V trap handling, PLIC for external interrupts, timer interrupts
- **Networking** — UDP stack with ARP, IPv4, Ethernet framing, VirtIO network driver, per-port socket binding
- **Drivers** — VirtIO network device (feature negotiation, virtqueues, packet TX/RX), PCI enumeration with MMIO BAR allocation and device tree parsing
- **Debugging** — DWARF-based backtrace with symbol resolution, state dump on Ctrl+D (heap stats, page allocator usage, process table), configurable per-module debug logging

### Userspace

- **Shell (SeSH)** — Command parsing, program execution with arguments, background processes (`&`), built-in help
- **12 programs** — init, shell, UDP networking, sleep, stress testing, game board, and various test utilities
- Programs are compiled against musl libc and embedded directly into the kernel binary

### Infrastructure

- **MCP server** — AI agents can boot QEMU, send shell commands, build the kernel, and run tests over the Model Context Protocol
- **GDB MCP server** — Programmatic GDB debugging (breakpoints, stepping, register inspection) exposed as MCP tools
- **System tests** — Integration tests that boot the OS in QEMU and interact via stdin/stdout, covering networking, processes, signals, stress, and shell behavior
- **Unit tests** — Kernel unit tests with a custom `#[test_case]` framework, plus Miri for undefined behavior detection
- **CI** — Build, fmt, clippy, unit tests, Miri, and system tests on self-hosted Nix runners

### Not Yet Implemented

- Filesystem (programs are embedded in the kernel image)
- TCP
- GUI
- DHCP / configurable IP addressing
- clone/execve (currently uses a custom sys_execute)

## How Do I Run It?

This project uses a Nix develop shell that provides all required tools.

```bash
# Install nix
sh <(curl -L https://nixos.org/nix/install) --daemon

# Enable nix-command and flakes
echo -e '\nexperimental-features = nix-command flakes\n' | sudo tee -a /etc/nix/nix.conf

# Restart nix daemon
sudo systemctl restart nix-daemon

# Install direnv
sudo apt install direnv

# Add direnv hook to your shell (see https://direnv.net/docs/hook.html for non-bash shells)
echo -e 'eval "$(direnv hook bash)"\n' >> ~/.bashrc

# In the SentientOS repository
direnv allow
# Re-enter the directory — nix will pull all dependencies automatically
```

Then run the OS:

```
just run
```

## What Can I Do?

Type `help` in the shell for available commands. Type a program name to execute it. Append `&` to run it in the background. See `userspace/src/bin/` for the full list of programs.

## Project Structure

```
kernel/           Main kernel (RISC-V 64-bit, no_std)
userspace/        Userspace programs (musl libc)
common/           Shared no_std library
system-tests/     Integration tests (run on x86, test via QEMU)
qemu-infra/       Shared QEMU communication library
mcp-server/       MCP server for AI agent interaction
gdb_mcp_server/   GDB MCP server for programmatic debugging
headers/          Linux C header bindings via bindgen
doc/ai/           Detailed AI-facing documentation
```

## Useful Commands

The project uses [just](https://github.com/casey/just) as a command runner. Run `just -l` for a full list.

```bash
just run            # Build and run in QEMU
just build          # Build kernel with userspace
just test           # Run all tests (unit + system)
just ci             # Run full CI pipeline (clippy, fmt, tests, miri)
just clippy         # Run linter
just debug          # Start QEMU with GDB in tmux
just addr2line ADDR # Resolve kernel address to source location
```
