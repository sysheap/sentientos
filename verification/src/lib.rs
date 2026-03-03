//! Formal verification proofs for Solaya kernel subsystems.
//!
//! This crate contains models of critical kernel data structures and
//! [Kani](https://model-checking.github.io/kani/) proof harnesses that
//! verify their correctness properties via bounded model checking.
//!
//! Each module mirrors the algorithm of a specific kernel subsystem,
//! abstracting away hardware details (raw pointers, MMIO) while
//! preserving the exact logic. Properties proved on the model hold
//! for the real implementation because the abstraction is sound:
//! pointer arithmetic is a bijection over indices, and bit operations
//! on `usize` are identical to those on pointer addresses.
//!
//! # Running proofs
//!
//! ```bash
//! # Install Kani (one-time)
//! cargo install --locked kani-verifier
//! cargo kani setup
//!
//! # Run all proofs
//! cd verification && cargo kani
//!
//! # Run a specific proof
//! cd verification && cargo kani --harness alloc_dealloc_roundtrip
//! ```
//!
//! # Running standard tests (no Kani needed)
//!
//! ```bash
//! cargo test -p verification
//! ```

pub mod page_allocator;
pub mod page_table_entry;
