# YaROS (Yet another RISC-V Operating System)
[![Build](https://github.com/hieronymusma/yaros/actions/workflows/build.yml/badge.svg)](https://github.com/hieronymusma/yaros/actions/workflows/build.yml)  
This projects makes my dream come true - write my own operating system. I'm doing this mostly for fun, so don't expect a fully-fledged operating system on basis of the RISC-V architecture.
## Status
Implemented
* Page allocator
* Heap allocator
* Interrupt handling (PLIC -> UART interrupts)
* Testing harness
* Executing in supervisor mode  

TODO
* Scheduler
* Userspace processes
* Systemcalls
* VirtIO / Filesystem
* Networkstack

## How do I run it?
To run the operating system you need to have the following tools installed:
* Rust
* Just (justr a command runner)
* qemu-system-riscv64  

To install them on Ubuntu you can execute the following commands
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install just
sudo apt install qemu-system-riscv64
```
To run the operating system execute
```
just run
```