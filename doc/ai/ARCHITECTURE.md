# Architecture

## Overview

SentientOS is a RISC-V 64-bit hobby OS kernel written in Rust. Key characteristics:
- Target: riscv64gc-unknown-none-elf (no_std)
- Virtual memory: Sv39 (3-level page tables)
- Multi-core (SMP) support via RISC-V SBI
- Async/await runtime for blocking syscalls
- Linux-compatible syscall interface

## Project Structure

```
kernel/src/
  main.rs              # Entry point, kernel_init
  cpu.rs               # Per-CPU state, CSR operations
  asm/                 # RISC-V assembly (context switch)
  memory/              # Page allocator, page tables
  processes/           # Process, thread, scheduler
  syscalls/            # Linux syscall handlers
  interrupts/          # Trap handling, PLIC
  net/                 # UDP network stack
  drivers/virtio/      # VirtIO drivers
  io/                  # UART, stdin buffer
  pci/                 # PCI enumeration
  sbi/                 # RISC-V SBI interface
  klibc/               # Kernel utilities
  debugging/           # Backtrace, symbols
  logging/             # Log macros

userspace/src/
  bin/                 # User programs (init, sesh, etc.)
  lib.rs               # Syscall wrappers
```

## Boot Sequence

Entry: `kernel_init(hart_id, device_tree_pointer)` in `kernel/src/main.rs:59`

```
kernel_init()
  |
  +-> QEMU_UART.init()              # Initialize serial output
  +-> sbi::get_spec_version()       # Check SBI version >= 0.2
  +-> sbi::get_number_of_harts()    # Count CPUs
  +-> symbols::init()               # Load debug symbols
  +-> device_tree::init()           # Parse device tree
  +-> memory::init_page_allocator() # Set up physical page allocator
  +-> backtrace::init()             # Initialize stack unwinding
  +-> timer::init()                 # Initialize timer subsystem
  +-> pci::parse()                  # Parse PCI from device tree
  +-> pci::PCI_ALLOCATOR_64_BIT.init()  # PCI address allocator
  +-> memory::initialize_runtime_mappings()  # Map PCI space
  +-> process_table::init()         # Create init process
  +-> Cpu::init()                   # Initialize boot CPU struct
  +-> Cpu::activate_kernel_page_table()
  +-> plic::init_uart_interrupt()   # Enable UART interrupts
  +-> enumerate_devices()           # Find PCI devices
  +-> NetworkDevice::initialize()   # Init VirtIO network
  +-> net::assign_network_device()  # Register network device
  +-> start_other_harts()           # Boot other CPUs
  +-> prepare_for_scheduling()      # Enter scheduler loop
```

`prepare_for_scheduling()` in `kernel/src/main.rs:144`:
```
prepare_for_scheduling()
  |
  +-> Cpu::write_sie(usize::MAX)    # Enable all interrupt sources
  +-> Cpu::csrs_sstatus(0b10)       # Enable global interrupts
  +-> timer::set_timer(0)           # Trigger immediate timer
  +-> wfi_loop()                    # Wait for interrupt loop
```

## CPU Structure

Per-CPU state in `kernel/src/cpu.rs:29`:

```rust
pub struct Cpu {
    kernel_page_tables_satp_value: usize,  # Kernel SATP for trap entry
    trap_frame: TrapFrame,                  # Saved registers on trap
    scheduler: Spinlock<CpuScheduler>,      # Per-CPU scheduler
    cpu_id: CpuId,                          # Hart ID
    kernel_page_tables: RootPageTableHolder,# Kernel page tables
    number_cpus: usize,                     # Total CPU count
}
```

Access current CPU: `Cpu::current()` (via sscratch CSR)

## Key Data Structures

### Process (kernel/src/processes/process.rs)
```rust
pub struct Process {
    name: String,
    page_table: RootPageTableHolder,        # Virtual address space
    allocated_pages: Vec<PinnedHeapPages>,  # Physical memory
    threads: BTreeMap<Tid, ThreadWeakRef>,  # Process threads
    brk: usize,                             # Heap break pointer
    free_mmap_address: usize,               # Next mmap address
    fd_table: FdTable,                     # File descriptor table
}
```

### Thread (kernel/src/processes/thread.rs)
```rust
pub struct Thread {
    tid: Tid,
    trap_frame: TrapFrame,                  # Saved registers
    program_counter: usize,                 # Current PC
    state: ThreadState,                     # Running/Runnable/Waiting
    process: ProcessRef,                    # Parent process
    syscall_task: Option<Task>,             # Async syscall task
    signal_state: SignalState,              # Signal handlers, mask, altstack
}
```

### TrapFrame (common/src/syscalls/trap_frame.rs)
All 32 general-purpose registers saved on trap/syscall.

## Subsystem Interactions

```
              Timer/External Interrupt
                      |
                      v
              interrupts/trap.rs
              (handle_interrupt)
                      |
          +-----------+-----------+
          |           |           |
          v           v           v
     Timer Int    UART Int    Syscall
          |           |           |
          v           v           v
    scheduler    stdin_buf   syscalls/
    .schedule()  .push()     handler.rs
          |           |           |
          v           v           v
    Context      read()      Process/
    Switch       wakes       Thread
                             state
```

## Memory Layout

### Kernel Virtual Address Space
- Kernel code/data: Identity-mapped from linker script
- Heap: After kernel image, size from device tree
- PCI ranges: Runtime-mapped from device tree
- Per-CPU kernel stack: Top of address space (0xFFFF...)

### User Virtual Address Space
- Code: 0x10000 (ELF load address)
- Stack: Top of address space growing down
- Heap (brk): After BSS, grows up
- mmap regions: Between heap and stack

## RISC-V Specifics

### CSRs Used
| CSR | Purpose |
|-----|---------|
| satp | Page table base register |
| sstatus | Supervisor status (interrupts, SPP) |
| sepc | Exception program counter |
| scause | Trap cause |
| stval | Trap value (bad address) |
| sscratch | Points to Cpu struct |
| sie | Interrupt enable bits |
| sip | Interrupt pending bits |

### SBI Interface
- `sbi::timer::set_timer()` - Schedule timer interrupt
- `sbi::hart_state::start_hart()` - Boot other CPUs
- `sbi::ipi::sbi_send_ipi()` - Inter-processor interrupt

### Page Table Format
Sv39: 39-bit virtual addresses, 3-level page tables
- VPN[2]: 9 bits (level 2)
- VPN[1]: 9 bits (level 1)
- VPN[0]: 9 bits (level 0)
- Page offset: 12 bits (4KB pages)

## Async Syscall Model

Blocking syscalls use Rust async/await:

1. Syscall invoked from userspace
2. Handler creates `Task` (async future)
3. Task polled in scheduler loop
4. If not ready: thread suspended, `Poll::Pending`
5. Waker registered (timer, I/O event)
6. Event occurs: waker called, thread marked runnable
7. Task polled again, returns `Poll::Ready`
8. Result returned to userspace

Key files:
- `kernel/src/processes/task.rs` - Task wrapper
- `kernel/src/processes/waker.rs` - ThreadWaker
- `kernel/src/syscalls/handler.rs` - SyscallHandler trait

## Key Files Quick Reference

| Purpose | File:Line |
|---------|-----------|
| Kernel entry | kernel/src/main.rs:59 |
| CPU struct | kernel/src/cpu.rs:29 |
| Trap entry | kernel/src/asm/trap.s |
| Trap handler | kernel/src/interrupts/trap.rs |
| Scheduler | kernel/src/processes/scheduler.rs |
| Process struct | kernel/src/processes/process.rs |
| Thread struct | kernel/src/processes/thread.rs |
| Syscall dispatch | kernel/src/syscalls/handler.rs |
| Page tables | kernel/src/memory/page_tables.rs |
