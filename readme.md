# YaROS (Yet another RISC-V Operating System)
[![ci](https://github.com/hieronymusma/yaros/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/hieronymusma/yaros/actions/workflows/ci.yml)  
This projects makes my dream come true - write my own operating system. I'm doing this mostly for fun, so don't expect a fully-fledged operating system on basis of the RISC-V architecture.
Exactly like [SerenityOS](https://github.com/SerenityOS/serenity) this project doesn't use third-party runtime dependencies. If third-party dependencies are used, then only for the Build.

## Status

Implemented

- Page allocator
- Heap allocator
- Interrupt handling (PLIC -> UART interrupts)
- Testing harness
- Executing in supervisor mode
- Userspace processes
- Scheduler
- Systemcalls
- Networkstack (udp)

TODO

- VirtIO / Filesystem
- TCP
- SMP
- Async Runtime in Kernel
- GUI
- See [todo](./todo.md)

## How do I run it?

To run the operating system you need to have the following tools installed:

- Rust
- just
- nextest
- qemu-system-riscv64
- binutils-riscv64-linux-gnu

To install them on Ubuntu you can execute the following commands

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install qemu-system-riscv64 binutils-riscv64-linux-gnu
cargo install just cargo-nextest --locked
```

To run the operating system execute

```
just run
```

## What can I do?

Type `help` into the shell to get some information. If you type the name of a program it get's executed. If you add an ampersand at the end of the command it get's executed in the background. See `src/userspace/src/bin` for programs which can be executed.

## Justfile

The justfile contains useful commands which I often use. To run them you first need to install just (just a command runner).
`cargo install just`. To get a list of all commands execute `just -l`.
