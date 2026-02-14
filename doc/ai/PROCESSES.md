# Process Management

## Overview

Process management consists of:
1. **Process** - Address space, threads, resources
2. **Thread** - Execution context, registers, state
3. **Scheduler** - Per-CPU round-robin scheduling
4. **Loader** - ELF binary loading

## Process Structure

**File:** `kernel/src/processes/process.rs:31`

```rust
pub struct Process {
    name: Arc<String>,
    page_table: RootPageTableHolder,           // Virtual address space
    allocated_pages: Vec<PinnedHeapPages>,     // Physical memory
    free_mmap_address: usize,                  // Next mmap VA (starts 0x2000000000)
    next_free_descriptor: u64,                 // UDP socket descriptor counter
    open_udp_sockets: BTreeMap<UDPDescriptor, SharedAssignedSocket>,
    threads: BTreeMap<Tid, ThreadWeakRef>,
    main_tid: Tid,
    brk: Brk,                                  // Heap break manager
}
```

### Key Methods

```rust
impl Process {
    // Memory management
    fn mmap_pages(&mut self, num_pages: usize, perm: XWRMode) -> *mut u8
    fn mmap_pages_with_address(&mut self, num_pages: usize, addr: usize, perm: XWRMode) -> *mut u8
    fn brk(&mut self, brk: usize) -> usize

    // Userspace pointer access
    fn read_userspace_ptr<T>(&self, ptr: &UserspacePtr<*const T>) -> Result<T, Errno>
    fn write_userspace_ptr<T>(&self, ptr: &UserspacePtr<*mut T>, value: T) -> Result<(), Errno>
    fn read_userspace_slice<T>(&self, ptr: &UserspacePtr<*const T>, len: usize) -> Result<Vec<T>, Errno>
    fn write_userspace_slice<T>(&self, ptr: &UserspacePtr<*mut T>, data: &[T]) -> Result<(), Errno>

    // UDP sockets
    fn put_new_udp_socket(&mut self, socket: SharedAssignedSocket) -> UDPDescriptor
    fn get_shared_udp_socket(&mut self, desc: UDPDescriptor) -> Option<&mut SharedAssignedSocket>
}
```

## Thread Structure

**File:** `kernel/src/processes/thread.rs:60`

```rust
pub struct Thread {
    tid: Tid,
    process_name: Arc<String>,
    register_state: TrapFrame,                 // All 32 GP registers
    program_counter: usize,                    // Current PC
    state: ThreadState,                        // Running/Runnable/Waiting
    in_kernel_mode: bool,                      // Kernel vs user mode
    process: ProcessRef,                       // Parent process
    notify_on_die: BTreeSet<Tid>,              // Threads to wake on exit
    clear_child_tid: Option<UserspacePtr<*mut c_int>>,
    signal_state: SignalState,                 // Signal handlers, mask, altstack
    syscall_task: Option<SyscallTask>,         // Pending async syscall
}
```

### Thread States

```rust
pub enum ThreadState {
    Running { cpu_id: usize },  // Currently executing on specified CPU
    Runnable,                   // Ready to run, in run queue
    Waiting,                    // Blocked (sleeping, waiting for I/O, or being killed)
}
```

The `cpu_id` in `Running` is critical for multi-CPU correctness. It ensures:
- A thread can only be scheduled on one CPU at a time
- The scheduler atomically claims threads by setting `Running { cpu_id }`
- Race conditions between CPUs are prevented (thread woken by waker on CPU1
  while CPU0 is about to return to userspace with it)

### Thread Creation

**From ELF:**
```rust
Thread::from_elf(elf_file: &ElfFile, name: &str, args: &[&str])
    -> Result<Arc<Spinlock<Thread>>, LoaderError>
```

**Powersave thread (idle):**
```rust
Thread::create_powersave_thread() -> Arc<Spinlock<Thread>>
```

## Scheduler

**File:** `kernel/src/processes/scheduler.rs:22`

Per-CPU scheduler with round-robin scheduling:

```rust
pub struct CpuScheduler {
    current_thread: ThreadRef,
    powersave_thread: ThreadRef,   // Idle thread (TID 0)
}
```

### Schedule Loop

`schedule()` is called on timer interrupt:

1. Save current thread state (PC, registers)
2. Set current thread to Runnable (if Running)
3. Get next runnable from process table
4. If thread has pending syscall task:
   - Poll the async task
   - If ready: write result to a0, skip ecall, return to userspace
   - If pending: thread stays in Waiting, try next
5. Load thread state (PC, registers)
6. Set timer (10ms normal, 50ms powersave)
7. Return to userspace via sret

### Timer Quantum

- Normal process: 10ms
- Powersave (idle): 50ms

### Key Scheduler Methods

```rust
impl CpuScheduler {
    // Get current thread/process
    fn get_current_thread(&self) -> &ThreadRef
    fn get_current_process(&self) -> ProcessRef

    // Schedule next process
    fn schedule(&mut self)

    // Start a new program
    fn start_program(&mut self, name: &str, args: &[&str]) -> Result<Tid, SchedulerError>

    // Kill current process
    fn kill_current_process(&mut self)

    // Handle Ctrl+C
    fn send_ctrl_c(&mut self)
}
```

## Process Table

**File:** `kernel/src/processes/process_table.rs`

Global registry of all threads:

```rust
pub static THE: Spinlock<ProcessTable> = Spinlock::new(ProcessTable::new());

struct ProcessTable {
    processes: BTreeMap<Tid, ThreadRef>,
    run_pointer: usize,              // Round-robin pointer
}
```

### Key Methods

```rust
impl ProcessTable {
    fn init()                                    // Create init process
    fn add_thread(&mut self, thread: ThreadRef)
    fn get_thread(&self, tid: Tid) -> Option<ThreadRef>
    fn next_runnable(&mut self) -> Option<ThreadRef>  // Get next runnable thread
    fn kill(&mut self, tid: Tid)
    fn is_empty(&self) -> bool
}
```

## ELF Loader

**File:** `kernel/src/processes/loader.rs`

Loads ELF binaries and sets up process address space:

```rust
pub fn load_elf(elf: &ElfFile, name: &str, args: &[&str])
    -> Result<LoadedElf, LoaderError>

pub struct LoadedElf {
    entry_address: usize,
    page_tables: RootPageTableHolder,
    allocated_pages: Vec<PinnedHeapPages>,
    args_start: usize,                    // Stack pointer with args
    brk: Brk,                             // Heap break manager
}
```

### User Memory Layout

```
0x10000             Entry point (ELF load address)
  .text             Code (RX)
  .data             Data (RW)
  .bss              BSS (RW, zeroed)
  brk_start         Heap start (grows up)
  ...
STACK_START         Stack grows down from here
STACK_END           Bottom of stack region
```

## Async Syscall Model

**File:** `kernel/src/processes/task.rs`

Blocking syscalls use Rust async/await:

```rust
pub type SyscallTask = Task<Result<isize, Errno>>;

pub struct Task<T> {
    future: Pin<Box<dyn Future<Output = T> + Send>>,
}
```

### Flow

1. Syscall handler creates async task
2. Scheduler polls task with ThreadWaker
3. If `Poll::Pending`: thread suspended, waker registered
4. When ready (timer, I/O): waker called, thread marked Runnable
5. Scheduler polls again, gets `Poll::Ready(result)`
6. Result written to a0, thread returns to userspace

### ThreadWaker

**File:** `kernel/src/processes/waker.rs`

```rust
impl ThreadWaker {
    pub fn new_waker(thread: ThreadWeakRef) -> Waker
}
```

When woken:
1. Upgrade weak reference to thread
2. Set thread state to Runnable
3. Thread will be scheduled on next `schedule()` call

## Brk (Heap Management)

**File:** `kernel/src/processes/brk.rs`

Manages process heap via brk syscall:

```rust
pub struct Brk {
    current: usize,   // Current break
    initial: usize,   // Initial break (after BSS)
}

impl Brk {
    pub fn brk(&mut self, new_brk: usize) -> usize
}
```

## UserspacePtr

**File:** `kernel/src/processes/userspace_ptr.rs`

Safe wrapper for userspace pointers:

```rust
pub struct UserspacePtr<P: Pointer>(P);

impl<P: Pointer> UserspacePtr<P> {
    pub fn new(ptr: P) -> Self             // Mark as userspace pointer
    pub unsafe fn get(&self) -> P          // Get raw pointer (unsafe)
}
```

Used with Process methods to safely read/write userspace memory.

## Key Files

| File | Purpose |
|------|---------|
| kernel/src/processes/process.rs | Process struct and methods |
| kernel/src/processes/thread.rs | Thread struct, state machine |
| kernel/src/processes/scheduler.rs | CpuScheduler, scheduling |
| kernel/src/processes/process_table.rs | Global process registry |
| kernel/src/processes/loader.rs | ELF loading |
| kernel/src/processes/task.rs | Async task wrapper |
| kernel/src/processes/waker.rs | ThreadWaker for async |
| kernel/src/processes/brk.rs | Heap break management |
| kernel/src/processes/timer.rs | Timer interrupt handling |
| kernel/src/processes/userspace_ptr.rs | Userspace pointer wrapper |

## Common Operations

### Start a New Program
```rust
Cpu::with_scheduler(|mut s| {
    s.start_program("prog1", &["arg1", "arg2"])?;
});
```

### Access Current Thread/Process
```rust
// Get current thread
let thread = Cpu::current_thread();

// Work with current process
Cpu::with_current_process(|process| {
    process.mmap_pages(1, XWRMode::ReadWrite);
});
```

### Create Async Syscall
```rust
async fn my_syscall(thread: ThreadWeakRef) -> Result<isize, Errno> {
    // Do async work
    timer::sleep(Duration::from_secs(1)).await;
    Ok(0)
}
```
