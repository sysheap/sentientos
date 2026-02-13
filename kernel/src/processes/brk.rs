use crate::{
    klibc::util::align_up_page_size,
    memory::{
        page::{Pages, PinnedHeapPages},
        page_tables::RootPageTableHolder,
    },
};

const BRK_SIZE: Pages = Pages(4);

#[derive(Debug)]
pub struct Brk {
    brk_start: usize,
    brk_current: usize,
    /// One past the end of the allocated area
    brk_end: usize,
}

impl Brk {
    pub fn new(bss_end: usize, page_tables: &mut RootPageTableHolder) -> (PinnedHeapPages, Self) {
        let brk_start = align_up_page_size(bss_end);
        let pages = PinnedHeapPages::new_pages(BRK_SIZE);
        page_tables.map_userspace(
            brk_start,
            pages.addr(),
            pages.size(),
            crate::memory::page_tables::XWRMode::ReadWrite,
            "BRK".into(),
        );
        let brk_end = brk_start + BRK_SIZE;
        (
            pages,
            Self {
                brk_start,
                brk_current: brk_start,
                brk_end,
            },
        )
    }

    pub fn empty() -> Self {
        Self {
            brk_start: 0,
            brk_current: 0,
            brk_end: 1,
        }
    }

    pub fn brk(&mut self, brk: usize) -> usize {
        if brk >= self.brk_start && brk < self.brk_end {
            self.brk_current = brk;
        }

        self.brk_current
    }
}

#[cfg(test)]
mod tests {
    use super::Brk;

    #[test_case]
    fn brk_within_range() {
        let mut brk = Brk {
            brk_start: 0x1000,
            brk_current: 0x1000,
            brk_end: 0x5000,
        };
        assert_eq!(brk.brk(0x2000), 0x2000);
        assert_eq!(brk.brk(0x4FFF), 0x4FFF);
    }

    #[test_case]
    fn brk_out_of_range_returns_current() {
        let mut brk = Brk {
            brk_start: 0x1000,
            brk_current: 0x2000,
            brk_end: 0x5000,
        };
        // Below start
        assert_eq!(brk.brk(0x0500), 0x2000);
        // At end (exclusive boundary)
        assert_eq!(brk.brk(0x5000), 0x2000);
        // Above end
        assert_eq!(brk.brk(0x9000), 0x2000);
    }

    #[test_case]
    fn brk_empty() {
        let mut brk = Brk::empty();
        assert_eq!(brk.brk(0), 0);
        // brk_end is 1, so 0 is within [0, 1)
        assert_eq!(brk.brk(0x1000), 0);
    }
}
