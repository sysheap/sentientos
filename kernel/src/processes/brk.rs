use common::util::align_up_page_size;

use crate::memory::{
    page::{Pages, PinnedHeapPages},
    page_tables::RootPageTableHolder,
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
