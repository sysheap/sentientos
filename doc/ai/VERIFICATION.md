# Formal Verification

## Overview

The `verification/` crate contains models of critical kernel data structures
and [Kani](https://model-checking.github.io/kani/) proof harnesses. Kani
performs bounded model checking: it exhaustively explores all possible inputs
up to a bound, proving properties hold universally (not just for tested cases).

Each module mirrors the algorithm of a kernel subsystem, abstracting away
hardware details (raw pointers, MMIO) while preserving the exact logic.

## What's Verified

### Page Allocator (`verification/src/page_allocator.rs`)

Models `kernel/src/memory/page_allocator.rs`. The real allocator uses a
metadata byte array and a pointer range; pointers are `base + index * PAGE_SIZE`
(a bijection), so we verify using indices only.

| Proof | Property |
|-------|----------|
| `alloc_marks_correctly` | alloc(n) sets exactly (n-1) Used + 1 Last |
| `alloc_dealloc_roundtrip` | alloc then dealloc restores Free state |
| `no_overlapping_allocations` | two allocs never return overlapping ranges |
| `exhaustion_detected` | can't allocate beyond capacity |
| `dealloc_count_matches_alloc` | dealloc returns the allocated count |
| `dealloc_order_independent` | two allocs freed in either order both work |
| `reallocation_after_free` | freed pages can be reallocated |
| `alloc_preserves_well_formed` | structural invariant maintained after alloc |
| `two_allocs_preserve_well_formed` | invariant holds after two allocs |
| `alloc_dealloc_preserves_well_formed` | invariant holds after mixed ops |

### Page Table Entry (`verification/src/page_table_entry.rs`)

Models `kernel/src/memory/page_table_entry.rs` and the bit utilities from
`kernel/src/klibc/util.rs`.

| Proof | Property |
|-------|----------|
| `bit_set_get_roundtrip` | set then get returns the set value |
| `bit_set_preserves_other_bits` | setting one bit doesn't affect others |
| `multiple_bits_roundtrip` | set/get multiple bits roundtrip |
| `validity_roundtrip` | set/get validity bit |
| `xwr_mode_roundtrip` | set/get XWR mode for all valid RISC-V modes |
| `user_mode_roundtrip` | set/get user-mode-accessible bit |
| `address_roundtrip` | set/get physical address for all 44-bit PPNs |
| `fields_are_independent` | changing one PTE field preserves all others |
| `is_leaf_iff_not_pointer` | is_leaf() matches XWR != PointerToNextLevel |
| `address_preserves_flags` | changing address preserves flag bits |

## Running

### Standard tests (no Kani needed)

```bash
just verify-test
# or: cargo test -p verification --target x86_64-unknown-linux-gnu
```

### Kani proofs (requires Kani)

```bash
# One-time install
cargo install --locked kani-verifier
cargo kani setup

# Run all proofs
just verify

# Run a specific proof
just verify-harness alloc_dealloc_roundtrip
```

## Design Decisions

**Why a separate crate?** The kernel crate targets `riscv64gc-unknown-none-elf`
with many nightly features and a custom test framework. Kani compiles for the
host and may not support all nightly features. A separate crate avoids these
conflicts while verifying the same algorithms.

**Why models, not the real code?** The models abstract raw pointer arithmetic
to array indices and `*mut PageTable` addresses to `usize` bit patterns. These
are sound abstractions: pointer math is a bijection over indices, and RISC-V PTE
bit operations on pointer addresses are identical to operations on `usize`.

**Why Kani?** Compared to alternatives:
- Kani harnesses look like Rust tests — low learning curve
- Supports `no_std` patterns, `MaybeUninit`, raw pointers, atomics
- Bounded model checking is fully automatic (no manual loop invariants)
- Maintained by Amazon, good Rust ecosystem integration

## Adding New Proofs

1. Identify the kernel algorithm to verify
2. Create a model in `verification/src/` that mirrors the exact logic
3. Add `#[cfg(kani)]` proof harnesses with `#[kani::proof]`
4. Add standard `#[test]` functions for quick validation without Kani
5. Document the verified properties in this file

### Choosing unwind bounds

`#[kani::unwind(K)]` limits loop unrolling. Set K to `MAX_SIZE + 2` where
MAX_SIZE is the largest const generic or loop bound in the proof. Start small
and increase if Kani reports "unwinding assertion" failures.

## Future Verification Targets

| Target | Kernel file | What to verify |
|--------|------------|----------------|
| Heap free-list | `memory/heap.rs` | Free-list integrity after alloc/dealloc sequences |
| Spinlock | `klibc/spinlock.rs` | Lock safety, no data races |
| Page table walk | `memory/page_tables.rs` | Correct address translation, no OOB |
| Thread state machine | `processes/thread.rs` | Valid state transitions |

## Key Files

| File | Purpose |
|------|---------|
| `verification/Cargo.toml` | Crate config with Kani check-cfg |
| `verification/src/lib.rs` | Module declarations |
| `verification/src/page_allocator.rs` | Page allocator model + proofs |
| `verification/src/page_table_entry.rs` | PTE bit manipulation model + proofs |
