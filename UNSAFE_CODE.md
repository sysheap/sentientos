# Unsafe Code Audit

## Summary

| Category | Count | Can eliminate? |
|----------|-------|---------------|
| Inline assembly (CSR, ecall, fence, wfi, rdtime) | ~15 | No |
| `#[unsafe(no_mangle)]` / `extern "C"` FFI | ~10 | No |
| Raw pointer deref (MMIO, page tables, heap) | ~20 | No (kernel fundamentals) |
| `slice::from_raw_parts` / `from_raw_parts_mut` | ~10 | No (kernel-userspace boundary) |
| `unsafe impl Send/Sync` | ~10 | Partially |
| `transmute` / `MaybeUninit::assume_init` | ~8 | Partially |
| `Box::from_raw` (page table drop) | 3 | No |
| `#[unsafe(naked)]` functions | 2 | No |
| Test-only unsafe | ~8 | Don't care |

Total production unsafe sites: ~70 (excluding tests)

---

## 1. Inline Assembly

### 1a. CSR read/write macros — `cpu.rs:37-86`

```rust
macro_rules! read_csrr { ... unsafe { asm!(concat!("csrr {}, ", ...) } }
macro_rules! write_csrr { ... unsafe { asm!(concat!("csrw ", ...) } }
```

**Instances:** read_satp, read_stval, read_sepc, read_scause, read_sscratch, read_sie, read_sstatus; write_satp, write_sepc, write_sscratch, write_sstatus, write_sie, write_sip (plus csrs_/csrc_ variants).

**Sound?** Yes. Each operates on a single CSR with correct RISC-V instruction syntax. No memory is touched. Miri-gated (`if cfg!(miri) { return }`).

**Can eliminate?** No. CSR access requires inline assembly; there is no safe abstraction possible.

### 1b. `write_satp_and_fence` — `cpu.rs:221-226`

```rust
pub unsafe fn write_satp_and_fence(satp_val: usize) {
    Cpu::write_satp(satp_val);
    unsafe { asm!("sfence.vma"); }
}
```

**Sound?** Yes, assuming the caller provides a valid SATP value. The function is correctly marked `unsafe fn`.

**Can eliminate?** No.

### 1c. `memory_fence` — `cpu.rs:228-232`

**Sound?** Yes. `fence` has no operands.

**Can eliminate?** No.

### 1d. `disable_global_interrupts` — `cpu.rs:234-237`

**Sound?** Yes. Correctly marked `unsafe fn`. Only called in panic handler and `wait_for_the_end`.

**Can eliminate?** No.

### 1e. `wait_for_interrupt` — `cpu.rs:239-243`

**Sound?** Yes. `wfi` is a hint instruction.

**Can eliminate?** No. Could arguably be safe (wfi is a hint), but keeping it unsafe is conservative and fine.

### 1f. `rdtime` — `processes/timer.rs:107`

**Sound?** Yes. Reads the time CSR into a register.

**Can eliminate?** No.

### 1g. `asm_panic_rust` — `asm/mod.rs:13`

**Sound?** Yes. `mv {}, ra` reads the return address register.

**Can eliminate?** No.

### 1h. SBI ecall — `sbi/sbi_call.rs:62-74`

**Sound?** Yes. Standard SBI calling convention, correct register constraints.

**Can eliminate?** No.

### 1i. Userspace ecall — `common/src/syscalls/macros.rs:21-28`

**Sound?** Yes. Standard ecall for system calls from userspace.

**Can eliminate?** No.

---

## 2. FFI / `#[unsafe(no_mangle)]` / `extern "C"`

### 2a. Trap handlers — `interrupts/trap.rs:19,24,30,199,208,217`

`get_process_satp_value`, `handle_timer_interrupt`, `handle_external_interrupt`, `handle_exception`, `handle_supervisor_software_interrupt`, `handle_unimplemented`

**Sound?** Yes. Called from `trap.S` assembly. The Rust functions themselves contain no unsafe operations — only the `#[unsafe(no_mangle)]` attribute is unsafe.

**Can eliminate?** No. Assembly must call these by symbol name.

### 2b. `kernel_init` / `prepare_for_scheduling` — `main.rs:59,147`

**Sound?** Yes. Entry points called from `boot.S`.

**Can eliminate?** No.

### 2c. `wfi_loop` — `asm/mod.rs:19-29`

`#[unsafe(naked)]` function with inline WFI loop.

**Sound?** Yes. Pure assembly, no memory access.

**Can eliminate?** No.

### 2d. `powersave` extern — `processes/thread.rs:117`

```rust
unsafe extern "C" { fn powersave(); }
```

**Sound?** Yes. Symbol provided by `powersave.S`.

**Can eliminate?** No.

### 2e. Linker symbols — `memory/linker_information.rs:5`

```rust
unsafe extern "C" { static $name: usize; }
```

**Sound?** Yes. `addr_of!` takes the address of a linker-provided symbol.

**Can eliminate?** No.

---

## 3. `unsafe impl Send / Sync`

### 3a. `Spinlock<T: Send>` — `klibc/spinlock.rs:123-124`

```rust
unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}
```

**Sound?** Yes. The spinlock provides mutual exclusion through an atomic CAS loop. `T: Send` is the correct bound. This mirrors `std::sync::Mutex`.

**Can eliminate?** No. This is the standard pattern for interior mutability containers.

### 3b. `RuntimeInitializedData<T>` — `klibc/runtime_initialized.rs:8`

```rust
unsafe impl<T> Sync for RuntimeInitializedData<T> {}
```

**Sound?** Yes. Initialization is guarded by an `AtomicBool` swap. After initialization, the data is read-only (returned via `Deref`). The `Sync` impl is justified because `initialize` can only succeed once (panics on double-init), and `Deref` only returns `&T` after checking the atomic flag.

**Can eliminate?** No. `UnsafeCell` prevents auto-Sync.

**Concern:** The bound should arguably be `T: Sync` — if `T` itself is not `Sync`, sharing `&RuntimeInitializedData<T>` across threads would allow concurrent `&T` access to a non-Sync type. In practice all uses are with `&str`, `Spinlock<Plic>`, `Backtrace`, and `usize`, all of which are `Sync`. **Low risk but technically unsound without `T: Sync` bound.**

### 3c. `MMIO<T>` — `klibc/mmio.rs:77`

```rust
unsafe impl<T> Send for MMIO<T> {}
```

**Sound?** Yes. MMIO wraps a raw pointer to a hardware register. The pointer is never dereferenced concurrently because MMIO objects are always behind a `Spinlock`. There's no `Sync` impl, which is correct.

**Can eliminate?** No. Raw pointers are `!Send`.

### 3d. `Uart` — `io/uart.rs:31-32`

```rust
unsafe impl Sync for Uart {}
unsafe impl Send for Uart {}
```

**Sound?** Yes. `Uart` contains an `MMIO` (raw pointer) and a `bool`. It's behind `QEMU_UART: Spinlock<Uart>`, so access is serialized.

**Can eliminate?** No. Needed because `MMIO` contains a raw pointer.

### 3e. `RootPageTableHolder` — `memory/page_tables.rs:85`

```rust
unsafe impl Send for RootPageTableHolder {}
```

**Sound?** Yes. Contains a raw pointer to page table memory. The holder is only moved between threads when a process migrates between CPUs. Access is serialized by the scheduler.

**Can eliminate?** No.

### 3f. `MetadataPageAllocator` — `memory/page_allocator.rs:35`

```rust
unsafe impl Send for MetadataPageAllocator<'_> {}
```

**Sound?** Yes. Contains raw pointers to the page allocator's heap region. Always behind a `Spinlock`.

**Can eliminate?** No.

### 3g. `Heap<Allocator>` — `memory/heap.rs:253`

```rust
unsafe impl<Allocator: PageAllocator> Send for Heap<Allocator> {}
```

**Sound?** Yes. Always behind `Spinlock`.

**Can eliminate?** No.

### 3h. `UserspacePtr<PTR>` — `processes/userspace_ptr.rs:7`

```rust
unsafe impl<PTR: Pointer> Send for UserspacePtr<PTR> {}
```

**Sound?** Yes. `UserspacePtr` wraps a raw pointer from userspace. It's validated before use.

**Can eliminate?** No.

### 3i. `ContainsUserspacePtr<T>` — `processes/userspace_ptr.rs:36`

**Sound?** Yes. Same rationale.

**Can eliminate?** No.

### 3j. `LinuxUserspaceArg<T>` — `syscalls/linux_validator.rs:9`

```rust
unsafe impl<T> Send for LinuxUserspaceArg<T> {}
```

**Sound?** Yes. Wraps a `usize` argument and a `ProcessRef` (Arc). The usize is just a numeric value representing a userspace address — not actually a pointer in Rust's ownership model.

**Can eliminate?** No. `PhantomData<T>` with pointer types causes `!Send`.

### 3k. `DeconstructedVec` — `drivers/virtio/virtqueue.rs:26`

```rust
unsafe impl Send for DeconstructedVec {}
```

**Sound?** Yes. Stores the raw parts of a `Vec<u8>` (ptr, len, cap). Since `Vec<u8>: Send`, and the raw parts represent the same data, this is correct.

**Can eliminate?** No. Raw pointer makes it `!Send`.

---

## 4. Raw Pointer Dereferences

### 4a. `Cpu::current()` — `cpu.rs:152-155`

```rust
pub fn current() -> &'static Cpu {
    unsafe { &*Self::cpu_ptr() }
}
```

**Sound?** Yes. `cpu_ptr()` reads `sscratch` CSR which was set to a `Box::leak`-ed `Cpu` during `init()`. The assertion in `cpu_ptr()` checks non-null and alignment. The `'static` lifetime is correct because the `Cpu` is leaked and never freed (Drop panics).

**Can eliminate?** No.

### 4b. `Cpu::read_trap_frame` / `write_trap_frame` — `cpu.rs:157-175`

**Sound?** Yes. Uses `byte_add(TRAP_FRAME_OFFSET)` computed via `offset_of!`. The `Cpu` struct is `#[repr(C)]`-less but `offset_of!` works correctly on non-repr(C) types since Rust 1.77. Uses `read_volatile`/`write_volatile` which is correct since trap handlers may modify the trap frame concurrently.

**Can eliminate?** No. Could access via `Cpu::current()` instead of manual offset arithmetic, which would be safer. **Improvement opportunity:** replace `byte_add` + `read_volatile` with `&Cpu::current().trap_frame` using `read_volatile` on the field — but this would require the field to be accessible, and volatile semantics are needed because the trap handler (assembly) writes to it.

### 4c. `Cpu::maybe_kernel_page_tables` — `cpu.rs:195-202`

**Sound?** Yes. Checks null and alignment before dereferencing.

**Can eliminate?** No. Needs raw pointer access to sscratch before Cpu is fully initialized.

### 4d. `Cpu::cpu_id()` — `cpu.rs:205-211`

```rust
unsafe { *addr_of!((*ptr).cpu_id) }
```

**Sound?** Yes. Uses `addr_of!` to avoid creating a reference to the whole `Cpu` struct. The pointer was validated as non-null.

**Can eliminate?** No. Same pattern as above — needed during early boot.

### 4e. Page table `table()` / `table_mut()` — `page_tables.rs:139-147`

**Sound?** Yes. The raw pointer is set during construction via `Box::leak` and never nulled until `Drop`. The `Drop` impl checks `is_active()` to prevent use-after-free of the page table that's currently in hardware.

**Can eliminate?** No. Page tables must be raw pointers because they're shared with hardware (SATP register).

### 4f. `PageTableEntry::get_target_page_table` — `page_table_entry.rs:121-126`

```rust
pub(super) fn get_target_page_table(&self) -> &'static mut PageTable {
    assert!(!self.is_leaf());
    assert!(!self.get_physical_address().is_null());
    unsafe { &mut *physical_address }
}
```

**Sound?** Conditionally. Assertions guard against null and leaf entries. However, the `'static mut` reference is concerning — multiple call sites can get `&mut` to the same page table simultaneously. In practice this doesn't cause issues because the page table holder has `&mut self` on all mutating operations, but **the `'static mut` return is technically unsound** if anyone can call this concurrently.

**Can eliminate?** No, but the `'static mut` lifetime should ideally be tied to the `RootPageTableHolder`'s lifetime. Low risk in practice because all callers hold `&mut RootPageTableHolder`.

### 4g. `Box::from_raw` in page table Drop — `page_tables.rs:121-125`

**Sound?** Yes. The page tables were created via `Box::leak(Box::new(...))`. The `Drop` walks the tree and frees all levels. The `is_active()` assertion prevents dropping a page table while it's loaded in hardware.

**Can eliminate?** No. Manual memory management is required because page tables are shared with hardware.

### 4h. `activate_page_table` — `page_tables.rs:524-526`

**Sound?** Yes. Calls `write_satp_and_fence` which is already `unsafe fn`. The caller computes a valid SATP value from the page table's physical address.

**Can eliminate?** No.

---

## 5. Slice Construction from Raw Parts

### 5a. Userspace pointer validation — `processes/process.rs:96,108,120`

```rust
let slice = unsafe { core::slice::from_raw_parts(kernel_ptr, len) };
```

**Sound?** Yes. `get_kernel_space_fat_pointer()` translates the userspace virtual address to a kernel physical address after validating that (1) it's a valid userspace address, (2) all pages in the range are mapped, (3) permissions match. The resulting kernel pointer is valid.

**Can eliminate?** No. Kernel must read/write userspace memory.

### 5b. Syscall validator — `syscalls/validator.rs:53,64,75,91,98-99`

Same pattern as 5a but via the new-style syscall path. `validate_and_translate_slice_ptr` checks page table mappings.

**Sound?** Yes.

**Can eliminate?** No.

### 5c. Device tree parsing — `device_tree.rs:89-98,109,125`

**Sound?** Yes, with caveats. The device tree pointer comes from the bootloader (OpenSBI). Validation checks: magic number, version, alignment, size bounds. The code trusts firmware-provided data after these checks.

**Can eliminate?** No. Must parse firmware-provided binary data.

### 5d. ELF parsing — `klibc/elf.rs:256,274`

```rust
unsafe { &*(self.data.as_ptr() as *const ElfHeader) }
```

**Sound?** Yes. The data is alignment-checked (`assert_eq!(data.as_ptr() as usize % 8, 0)`), size-checked, and validity-checked before any cast. `#[repr(C)]` ensures layout compatibility.

**Can eliminate?** No. Must parse binary ELF headers.

### 5e. `eh_frame` backtrace — `debugging/backtrace.rs:59`

**Sound?** Yes. Linker symbols provide the start and size. The data is read-only.

**Can eliminate?** No.

### 5f. `Page::as_u8_slice` — `memory/page.rs:53`

**Sound?** Yes. Reinterprets a `&mut [Page]` as `&mut [u8]`. Valid because `Page` is `repr(C, align(4096))` and `u8` has alignment 1.

**Can eliminate?** No.

### 5g. `ArrayVec` Deref — `klibc/array_vec.rs:107,114`

**Sound?** Yes. `MaybeUninit<T>` has the same layout as `T`. Only the first `self.len()` elements are exposed, and `len()` is maintained as an invariant of `push`/`pop`.

**Can eliminate?** No. Standard `MaybeUninit`-based collection pattern.

### 5h. `BufferExtension::interpret_as` — `klibc/util.rs:61`

**Sound?** Yes, if the caller ensures the data is valid `T`. Alignment and size are checked by assertions. Used for network packet parsing where the buffer comes from the device.

**Can eliminate?** No.

### 5i. `ByteInterpretable::as_slice` — `klibc/util.rs:82`

**Sound?** Yes. Any `Sized` type can be viewed as bytes.

**Can eliminate?** No.

### 5j. `init_page_allocator` — `memory/mod.rs:70`

```rust
let memory = unsafe { from_raw_parts_mut(heap_start as *mut MaybeUninit<u8>, heap_size) };
```

**Sound?** Yes. `heap_start` comes from the linker symbol `__start_heap`. The size is computed from linker symbols. This is initialized exactly once during boot.

**Can eliminate?** No.

---

## 6. Transmute / MaybeUninit

### 6a. `XWRMode::from(u8)` — `page_table_entry.rs:19-23`

```rust
unsafe { core::mem::transmute(value) }
```

**Sound?** **UNSOUND.** The enum has 6 variants (0,1,3,4,5,7) but the transmute accepts any `u8`. Values 2 and 6 (and 8-255) have no variant, creating undefined behavior. In practice, only values extracted from page table entries are passed, which should always be valid. But the code has no validation.

**Can eliminate?** Yes! Replace with a `match` statement or use `TryFrom`. This is the most clearly improvable unsafe in the codebase.

### 6b. `SbiRet::new` — `sbi/sbi_call.rs:28-35`

```rust
unsafe { core::mem::transmute::<i64, SbiError>(error) }
```

**Sound?** Conditionally. SBI spec guarantees the error codes, but a buggy firmware could return unexpected values. The function is `unsafe fn` which is appropriate. In practice OpenSBI always returns valid values.

**Can eliminate?** Yes. Replace with `TryFrom<i64>` or a match. **Improvement opportunity.**

### 6c. Page allocator `align_to_mut` + `transmute` — `page_allocator.rs:64,68,81-82`

```rust
let (begin, metadata, end) = unsafe { metadata.align_to_mut::<MaybeUninit<PageStatus>>() };
```

**Sound?** Yes. `align_to_mut` is unsafe because it reinterprets memory. The assertions check that `begin` and `end` are empty (perfect alignment). The subsequent `transmute` from `&mut [MaybeUninit<PageStatus>]` to `&mut [PageStatus]` is valid because all elements are initialized in the loop on lines 76-78.

**Can eliminate?** The `transmute` could be replaced with `MaybeUninit::slice_assume_init_mut()` (nightly) or kept as-is. The `align_to_mut` calls are necessary.

### 6d. `MaybeUninit::assume_init` — `common/src/syscalls/macros.rs:35-37`

```rust
unsafe { ret.assume_init() }
```

**Sound?** Yes. The kernel writes to the `ret` pointer during syscall dispatch (line 67). If the syscall succeeded (checked on line 32), the return value is initialized.

**Can eliminate?** No. Standard `MaybeUninit` pattern for syscall returns.

### 6e. Syscall dispatch pointer deref — `common/src/syscalls/macros.rs:64-65`

```rust
let (arg_ref, ret_ref) = unsafe { (&*arg_ptr, &mut *ret_ptr) };
```

**Sound?** Yes. Both pointers were validated by `validate_and_translate_pointer` on lines 60-62.

**Can eliminate?** No.

### 6f. `RuntimeInitializedData` initialize/deref — `klibc/runtime_initialized.rs:26-28,40`

**Sound?** Yes. `initialize` writes via `UnsafeCell::get()` and the `AtomicBool` ensures only one initialization. `Deref` checks the flag before calling `assume_init_ref()`.

**Can eliminate?** No. Core building block for kernel statics.

### 6g. `ArrayVec` MaybeUninit operations — `klibc/array_vec.rs:36,65-66,86`

**Sound?** Yes. The `length` field tracks how many elements are initialized. `pop` decrements before reading; `drop` only drops `0..length`; iterator only reads `< length`.

**Can eliminate?** No.

---

## 7. MMIO Operations

### 7a. `MMIO::read` / `MMIO::write` — `klibc/mmio.rs:39-47`

```rust
pub fn read(&self) -> T { unsafe { self.addr.read_volatile() } }
pub fn write(&mut self, value: T) { unsafe { self.addr.write_volatile(value); } }
```

**Sound?** Yes, assuming the address points to a valid MMIO register. All MMIO addresses are hardcoded constants or derived from PCI/device tree configuration.

**Can eliminate?** No. Volatile access to hardware registers requires unsafe.

### 7b. `MMIO::add` / `new_type` / `new_type_with_offset` — `klibc/mmio.rs:17-35`

**Sound?** Yes. These are unsafe fns that do pointer arithmetic. Callers (mmio_struct macro, PCI code) provide correct offsets via `offset_of!`.

**Can eliminate?** No.

### 7c. `mmio_struct!` macro field accessors — `klibc/mmio.rs:112`

**Sound?** Yes. Uses `offset_of!` to compute correct field offsets.

**Can eliminate?** No.

### 7d. PLIC `set_priority` — `interrupts/plic.rs:36-40`

```rust
unsafe { self.priority_register_base.add(interrupt_id as usize).write(priority); }
```

**Sound?** Yes. `add` advances by `interrupt_id * sizeof(u32)` from the PLIC base. Interrupt ID bounds are controlled by the caller (always `UART_INTERRUPT_NUMBER = 10`).

**Can eliminate?** Could be made safe by using MMIO array indexing (like `MMIO<[u32; N]>`). **Minor improvement opportunity.**

---

## 8. Heap Allocator — `memory/heap.rs`

### 8a. `FreeBlock::initialize` — `heap.rs:93-95`

**Sound?** Yes. Writes a `FreeBlock` to a `NonNull` pointer. The pointer comes from the page allocator or from splitting a larger block.

### 8b. `FreeBlock::split` — `heap.rs:102,112`

**Sound?** Yes. `as_mut()` on a valid `NonNull`. `byte_add` advances within the block's allocation, guarded by size assertions.

### 8c. `Heap::dealloc` — `heap.rs:189-200`

**Sound?** Yes. `NonNull::new_unchecked` is safe because `dealloc` asserts `!ptr.is_null()` on line 186.

### 8d. `Heap::insert` / `split_if_necessary` / `find_and_remove` — `heap.rs:206,217,233`

**Sound?** Yes. All operate on `NonNull<FreeBlock>` pointers that are maintained as a linked list invariant.

### 8e. `GlobalAlloc` impl — `heap.rs:263-270`

**Sound?** Yes. Required by the `GlobalAlloc` trait. The spinlock ensures mutual exclusion.

**Can eliminate any of 8a-8e?** No. Heap allocators inherently require unsafe pointer manipulation.

---

## 9. PCI — `pci/mod.rs`

### 9a. `PCIDevice::try_new` — `pci/mod.rs:126`

```rust
unsafe fn try_new(address: usize) -> Option<Self> {
```

**Sound?** Yes. The address is computed from PCI configuration space enumeration. The function is `unsafe fn`. MMIO reads at invalid addresses return 0xFFFF (checked as `INVALID_VENDOR_ID`).

**Can eliminate?** No.

### 9b. PCI capability iterator — `pci/mod.rs:101`

```rust
unsafe { self.pci_device.configuration_space.new_type_with_offset(...) }
```

**Sound?** Yes. Offset comes from the capability linked list in PCI config space.

**Can eliminate?** No.

---

## 10. VirtIO — `drivers/virtio`

### 10a. `DeconstructedVec::into_vec_with_len` — `virtqueue.rs:43`

```rust
unsafe { Vec::from_raw_parts(self.ptr, length, self.capacity) }
```

**Sound?** Yes. The vec was deconstructed via `into_raw_parts()` and reconstructed with a potentially different length (from the device). The assertion `length <= capacity` prevents buffer overflow.

**Can eliminate?** No. Need to pass buffer ownership to hardware and get it back with a different length.

### 10b. VirtIO net `new_type` calls — `drivers/virtio/net/mod.rs:66,92`

**Sound?** Yes. Casts PCI capability MMIO to VirtIO-specific types at known offsets.

**Can eliminate?** No.

---

## 11. Panic Handler — `panic.rs`

### 11a. `disable_global_interrupts` — `panic.rs:21-23`

**Sound?** Yes. Necessary to prevent re-entrant interrupts during panic.

### 11b. `QEMU_UART.force_unlock()` — `panic.rs:37-38`

**Sound?** Yes. During panic, the UART lock may be held by the panicking code. Force-unlocking allows the panic message to be printed. This is a last-resort mechanism.

**Can eliminate?** No. Essential for panic diagnostics.

---

## 12. Symbols — `debugging/symbols.rs:12`

```rust
let cstr = unsafe { core::ffi::CStr::from_ptr(symbols_start as *const c_char) };
```

**Sound?** Yes. The symbol table is embedded by the build system and null-terminated. The linker symbol provides the start address.

**Can eliminate?** No.

---

## 13. Spinlock Internals — `klibc/spinlock.rs`

### 13a. `SpinlockGuard` Deref/DerefMut — `spinlock.rs:142,149`

**Sound?** Yes. The guard holds the lock (atomic CAS succeeded). `UnsafeCell::get()` is the only way to access the inner data. Exclusive access is guaranteed by the lock.

### 13b. `force_unlock` — `spinlock.rs:117-120`

**Sound?** Yes when used correctly. It's `unsafe fn` and documented as "only safe during panic when the holder will never resume." Only called from the panic handler.

**Can eliminate?** No.

---

## 14. Backtrace — `debugging/backtrace.rs`

### 14a. `ptr.read()` in unwinder — `backtrace.rs:101`

**Sound?** Yes. Reads a saved register value from the stack at a CFA-relative offset computed from DWARF unwind info.

**Can eliminate?** No.

### 14b. `#[unsafe(naked)]` dispatch — `backtrace.rs:279`

**Sound?** Yes. The naked function saves callee-saved registers to a struct, calls the closure, then restores them. Standard pattern from the `unwinding` crate.

**Can eliminate?** No.

### 14c. Test callback cast — `backtrace.rs:383`

Test-only. Not a concern.

---

## Actionable Findings

### Must Fix (Unsound)

1. **`XWRMode::from(u8)` transmute** (`page_table_entry.rs:21`): Replace with a `match` or `TryFrom`. Values 2, 6, 8-255 are undefined.

### Should Fix (Improvable)

2. **`SbiRet::new` transmute** (`sbi/sbi_call.rs:29`): Replace with a `match`. Low risk but easy to fix.

3. **`RuntimeInitializedData` missing `T: Sync` bound** (`runtime_initialized.rs:8`): Add `T: Sync` bound to the `unsafe impl Sync`. Currently all users happen to use Sync types, but the blanket impl is technically unsound.

### Consider (Low Priority)

4. **`Cpu::read/write_trap_frame` manual offset arithmetic** (`cpu.rs:157-175`): Could access via `Cpu::current()` struct field instead of raw pointer + `byte_add`. Would eliminate 2 unsafe blocks. However, volatile semantics may be needed due to concurrent trap handler access.

5. **PLIC `set_priority` pointer arithmetic** (`plic.rs:36-40`): Could use MMIO array type instead.
