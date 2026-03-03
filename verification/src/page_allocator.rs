//! Model and formal proofs for the page allocator.
//!
//! Mirrors the allocation algorithm from `kernel/src/memory/page_allocator.rs`,
//! abstracting physical memory pointers to array indices. The pointer math in
//! the real allocator is `base + index * PAGE_SIZE` — a bijection — so properties
//! proved on indices hold for the real implementation.
//!
//! # Verified properties
//!
//! - **alloc_marks_correctly**: alloc(n) sets exactly (n-1) Used + 1 Last
//! - **alloc_dealloc_roundtrip**: alloc then dealloc restores all pages to Free
//! - **no_overlapping_allocations**: two allocs never return overlapping ranges
//! - **exhaustion_detected**: can't allocate beyond capacity
//! - **dealloc_count_matches_alloc**: dealloc returns the allocated count
//! - **dealloc_order_independent**: two allocs freed in either order both work
//! - **reallocation_after_free**: freed pages can be reallocated
//! - **alloc_preserves_well_formed**: metadata structural invariant maintained
//! - **two_allocs_preserve_well_formed**: invariant holds after two allocs
//! - **alloc_dealloc_preserves_well_formed**: invariant holds after mixed ops

/// Page metadata state. Mirrors `kernel/src/memory/page_allocator.rs:14`.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PageStatus {
    Free,
    Used,
    Last,
}

impl PageStatus {
    fn is_free(&self) -> bool {
        matches!(self, Self::Free)
    }
}

/// Abstract model of `MetadataPageAllocator`.
///
/// The real allocator stores a metadata slice and a pointer range to physical
/// pages. Since `page_ptr = base + index * PAGE_SIZE` is a bijection, we only
/// need the metadata array to verify the algorithm's correctness.
pub struct PageAllocatorModel<const N: usize> {
    pub metadata: [PageStatus; N],
}

impl<const N: usize> Default for PageAllocatorModel<N> {
    fn default() -> Self {
        Self {
            metadata: [PageStatus::Free; N],
        }
    }
}

impl<const N: usize> PageAllocatorModel<N> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mirrors `MetadataPageAllocator::alloc` (`page_allocator.rs:131`).
    ///
    /// Linear scan for the first contiguous range of `number_of_pages` Free
    /// pages, then marks them as Used/Last.
    pub fn alloc(&mut self, number_of_pages: usize) -> Option<usize> {
        assert!(number_of_pages > 0, "Cannot allocate zero pages");
        if number_of_pages > N {
            return None;
        }
        let start_idx = (0..=(N - number_of_pages))
            .find(|&idx| self.is_range_free(idx, number_of_pages));
        if let Some(idx) = start_idx {
            self.mark_range_as_used(idx, number_of_pages);
        }
        start_idx
    }

    /// Mirrors `MetadataPageAllocator::dealloc` (`page_allocator.rs:204`).
    ///
    /// Walks from `start` until the Last marker, marking everything Free.
    pub fn dealloc(&mut self, start: usize) -> usize {
        assert!(
            self.metadata[start] == PageStatus::Used
                || self.metadata[start] == PageStatus::Last,
            "Double-free detected: page at index {start} has status {:?}",
            self.metadata[start]
        );
        let mut count = 0;
        let mut idx = start;
        while self.metadata[idx] != PageStatus::Last {
            self.metadata[idx] = PageStatus::Free;
            idx += 1;
            count += 1;
        }
        self.metadata[idx] = PageStatus::Free;
        count += 1;
        count
    }

    /// Mirrors `MetadataPageAllocator::is_range_free` (`page_allocator.rs:150`).
    fn is_range_free(&self, start: usize, count: usize) -> bool {
        (start..start + count).all(|i| self.metadata[i].is_free())
    }

    /// Mirrors `MetadataPageAllocator::mark_range_as_used` (`page_allocator.rs:154`).
    fn mark_range_as_used(&mut self, start: usize, count: usize) {
        for idx in start..start + count {
            let status = if idx == start + count - 1 {
                PageStatus::Last
            } else {
                PageStatus::Used
            };
            self.metadata[idx] = status;
        }
    }

    /// Structural invariant: metadata represents well-formed allocations.
    ///
    /// Every contiguous non-Free region must consist of zero or more Used
    /// pages followed by exactly one Last page.
    pub fn is_well_formed(&self) -> bool {
        let mut in_allocation = false;
        for i in 0..N {
            match (in_allocation, self.metadata[i]) {
                (false, PageStatus::Free) => {}
                (false, PageStatus::Used) => in_allocation = true,
                (false, PageStatus::Last) => {} // single-page allocation
                (true, PageStatus::Free) => return false, // Used without Last
                (true, PageStatus::Used) => {} // mid-allocation
                (true, PageStatus::Last) => in_allocation = false,
            }
        }
        !in_allocation // must not end mid-allocation
    }
}

// ── Kani proof harnesses ──
//
// Run with: cd verification && cargo kani
// Each proof exhaustively checks all possible inputs up to the bound.

#[cfg(kani)]
mod proofs {
    use super::*;

    // 5 pages exercises all interesting patterns: single-page, multi-page,
    // fragmentation, interleaved alloc/dealloc.
    const MAX_PAGES: usize = 5;

    /// After alloc(n), metadata has exactly (n-1) Used + 1 Last at the
    /// returned position, and all other pages remain Free.
    #[kani::proof]
    #[kani::unwind(7)]
    fn alloc_marks_correctly() {
        let mut a = PageAllocatorModel::<MAX_PAGES>::new();
        let n: usize = kani::any();
        kani::assume(n > 0 && n <= MAX_PAGES);

        if let Some(start) = a.alloc(n) {
            // Correct Used/Last pattern
            for i in start..start + n - 1 {
                assert_eq!(a.metadata[i], PageStatus::Used);
            }
            assert_eq!(a.metadata[start + n - 1], PageStatus::Last);

            // Untouched pages remain Free
            for i in 0..start {
                assert_eq!(a.metadata[i], PageStatus::Free);
            }
            for i in (start + n)..MAX_PAGES {
                assert_eq!(a.metadata[i], PageStatus::Free);
            }
        }
    }

    /// alloc(n) followed by dealloc restores all pages to Free.
    #[kani::proof]
    #[kani::unwind(7)]
    fn alloc_dealloc_roundtrip() {
        let mut a = PageAllocatorModel::<MAX_PAGES>::new();
        let n: usize = kani::any();
        kani::assume(n > 0 && n <= MAX_PAGES);

        if let Some(start) = a.alloc(n) {
            let freed = a.dealloc(start);
            assert_eq!(freed, n);
            for i in 0..MAX_PAGES {
                assert_eq!(a.metadata[i], PageStatus::Free);
            }
        }
    }

    /// Two successful allocations never overlap.
    #[kani::proof]
    #[kani::unwind(7)]
    fn no_overlapping_allocations() {
        let mut a = PageAllocatorModel::<MAX_PAGES>::new();
        let n1: usize = kani::any();
        let n2: usize = kani::any();
        kani::assume(n1 > 0 && n1 <= MAX_PAGES);
        kani::assume(n2 > 0 && n2 <= MAX_PAGES);

        if let Some(s1) = a.alloc(n1) {
            if let Some(s2) = a.alloc(n2) {
                assert!(s1 + n1 <= s2 || s2 + n2 <= s1);
            }
        }
    }

    /// Allocating all pages then requesting one more fails.
    #[kani::proof]
    #[kani::unwind(7)]
    fn exhaustion_detected() {
        let mut a = PageAllocatorModel::<MAX_PAGES>::new();
        let result = a.alloc(MAX_PAGES);
        assert!(result.is_some());
        assert!(a.alloc(1).is_none());
    }

    /// dealloc returns the exact count that was allocated.
    #[kani::proof]
    #[kani::unwind(7)]
    fn dealloc_count_matches_alloc() {
        let mut a = PageAllocatorModel::<MAX_PAGES>::new();
        let n: usize = kani::any();
        kani::assume(n > 0 && n <= MAX_PAGES);

        if let Some(start) = a.alloc(n) {
            let freed = a.dealloc(start);
            assert_eq!(freed, n);
        }
    }

    /// Two allocations can be freed in either order.
    #[kani::proof]
    #[kani::unwind(7)]
    fn dealloc_order_independent() {
        let mut a = PageAllocatorModel::<MAX_PAGES>::new();
        let n1: usize = kani::any();
        let n2: usize = kani::any();
        kani::assume(n1 > 0 && n1 <= 2);
        kani::assume(n2 > 0 && n2 <= 2);

        if let Some(s1) = a.alloc(n1) {
            if let Some(s2) = a.alloc(n2) {
                let free_first_first: bool = kani::any();
                if free_first_first {
                    a.dealloc(s1);
                    a.dealloc(s2);
                } else {
                    a.dealloc(s2);
                    a.dealloc(s1);
                }
                for i in 0..MAX_PAGES {
                    assert_eq!(a.metadata[i], PageStatus::Free);
                }
            }
        }
    }

    /// Freed pages can be reallocated.
    #[kani::proof]
    #[kani::unwind(7)]
    fn reallocation_after_free() {
        let mut a = PageAllocatorModel::<MAX_PAGES>::new();
        let n: usize = kani::any();
        kani::assume(n > 0 && n <= MAX_PAGES);

        if let Some(start) = a.alloc(n) {
            a.dealloc(start);
            assert!(a.alloc(n).is_some());
        }
    }

    /// A single alloc preserves the metadata structural invariant.
    #[kani::proof]
    #[kani::unwind(7)]
    fn alloc_preserves_well_formed() {
        let mut a = PageAllocatorModel::<MAX_PAGES>::new();
        assert!(a.is_well_formed());

        let n: usize = kani::any();
        kani::assume(n > 0 && n <= MAX_PAGES);
        let _ = a.alloc(n);
        assert!(a.is_well_formed());
    }

    /// Two allocs preserve the metadata structural invariant.
    #[kani::proof]
    #[kani::unwind(7)]
    fn two_allocs_preserve_well_formed() {
        let mut a = PageAllocatorModel::<MAX_PAGES>::new();
        let n1: usize = kani::any();
        let n2: usize = kani::any();
        kani::assume(n1 > 0 && n1 <= MAX_PAGES);
        kani::assume(n2 > 0 && n2 <= MAX_PAGES);

        let _ = a.alloc(n1);
        let _ = a.alloc(n2);
        assert!(a.is_well_formed());
    }

    /// Alloc + alloc + dealloc preserves the metadata structural invariant.
    #[kani::proof]
    #[kani::unwind(7)]
    fn alloc_dealloc_preserves_well_formed() {
        let mut a = PageAllocatorModel::<MAX_PAGES>::new();
        let n1: usize = kani::any();
        let n2: usize = kani::any();
        kani::assume(n1 > 0 && n1 <= 2);
        kani::assume(n2 > 0 && n2 <= 2);

        let s1 = a.alloc(n1);
        let _ = a.alloc(n2);
        if let Some(start) = s1 {
            a.dealloc(start);
        }
        assert!(a.is_well_formed());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_alloc_dealloc() {
        let mut a = PageAllocatorModel::<8>::new();
        let start = a.alloc(3).unwrap();
        assert_eq!(start, 0);
        assert_eq!(a.metadata[0], PageStatus::Used);
        assert_eq!(a.metadata[1], PageStatus::Used);
        assert_eq!(a.metadata[2], PageStatus::Last);

        let freed = a.dealloc(start);
        assert_eq!(freed, 3);
        assert!(a.metadata.iter().all(|s| *s == PageStatus::Free));
    }

    #[test]
    fn exhaustion() {
        let mut a = PageAllocatorModel::<4>::new();
        assert!(a.alloc(4).is_some());
        assert!(a.alloc(1).is_none());
    }

    #[test]
    fn well_formed_after_operations() {
        let mut a = PageAllocatorModel::<8>::new();
        assert!(a.is_well_formed());
        a.alloc(2).unwrap();
        assert!(a.is_well_formed());
        a.alloc(3).unwrap();
        assert!(a.is_well_formed());
        a.dealloc(0);
        assert!(a.is_well_formed());
    }

    #[test]
    fn fragmentation_and_reuse() {
        let mut a = PageAllocatorModel::<8>::new();
        let s1 = a.alloc(2).unwrap();
        let _s2 = a.alloc(2).unwrap();
        a.dealloc(s1); // free first block, creating a hole
        let s3 = a.alloc(1).unwrap();
        assert_eq!(s3, 0); // reuses the hole
        assert!(a.is_well_formed());
    }

    #[test]
    #[should_panic(expected = "Double-free")]
    fn double_free_panics() {
        let mut a = PageAllocatorModel::<8>::new();
        let start = a.alloc(2).unwrap();
        a.dealloc(start);
        a.dealloc(start);
    }

    #[test]
    #[should_panic(expected = "Cannot allocate zero")]
    fn zero_alloc_panics() {
        let mut a = PageAllocatorModel::<4>::new();
        a.alloc(0);
    }
}
