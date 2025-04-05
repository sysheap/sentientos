# SentientOS
[![ci](https://github.com/sysheap/sentientos/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sysheap/sentientos/actions/workflows/ci.yml)  
This projects makes my dream come true - write my own operating system. I'm doing this mostly for fun, so don't expect a fully-fledged operating system on basis of the RISC-V architecture.
Exactly like [SerenityOS](https://github.com/SerenityOS/serenity) this project doesn't use third-party runtime dependencies. If third-party dependencies are used, then only for the Build.

I started doing some coding videos on Youtube. Go and check it out if you want [sysheap channel](http://www.youtube.com/@sysheap)

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
- SMP

TODO

- VirtIO / Filesystem
- TCP
- Async Runtime in Kernel
- GUI
- See [todo](./todo.md)

## How do I run it?
This project contains a nix develop shell which includes all the tools to build the operating system.
```bash
  # Install nix
  sh <(curl -L https://nixos.org/nix/install) --daemon
  # Enable nix-command and flakes
  echo -e '\nexperimental-features = nix-command flakes\n' | sudo tee -a /etc/nix/nix.conf
  # Restart nix daemon
  sudo systemctl restart nix-daemon
  # Install direnv
  sudo apt install direnv
  # Add direnv hook to your shell
  # see https://direnv.net/docs/hook.html for other shells than bash
  echo -e 'eval "$(direnv hook bash)"\n' >> ~/.bashrc
  # Got to the SentienOS repository
  direnv allow
  # Re-enter the repository and all the dependencies should be pulled by nix
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
