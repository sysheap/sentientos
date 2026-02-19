use crate::{
    klibc::util::align_up_page_size,
    memory::{
        page::{Pages, PinnedHeapPages},
        page_tables::RootPageTableHolder,
    },
};

const BRK_SIZE: Pages = Pages::new(4);

#[derive(Debug)]
pub struct Brk {
    brk_start: crate::memory::VirtAddr,
    brk_current: crate::memory::VirtAddr,
    /// One past the end of the allocated area
    brk_end: crate::memory::VirtAddr,
}

impl Brk {
    pub fn new(
        bss_end: crate::memory::VirtAddr,
        page_tables: &mut RootPageTableHolder,
    ) -> (PinnedHeapPages, Self) {
        let brk_start = crate::memory::VirtAddr::new(align_up_page_size(bss_end.as_usize()));
        let pages = PinnedHeapPages::new_pages(BRK_SIZE);
        page_tables.map_userspace(
            brk_start,
            crate::memory::PhysAddr::new(pages.addr()),
            pages.size(),
            crate::memory::page_tables::XWRMode::ReadWrite,
            "BRK".into(),
        );
        let brk_end = brk_start.add(BRK_SIZE.as_bytes());
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
            brk_start: crate::memory::VirtAddr::zero(),
            brk_current: crate::memory::VirtAddr::zero(),
            brk_end: crate::memory::VirtAddr::new(1),
        }
    }

    pub fn brk(&mut self, brk: crate::memory::VirtAddr) -> crate::memory::VirtAddr {
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
        use crate::memory::VirtAddr;
        let mut brk = Brk {
            brk_start: VirtAddr::new(0x1000),
            brk_current: VirtAddr::new(0x1000),
            brk_end: VirtAddr::new(0x5000),
        };
        assert_eq!(brk.brk(VirtAddr::new(0x2000)), VirtAddr::new(0x2000));
        assert_eq!(brk.brk(VirtAddr::new(0x4FFF)), VirtAddr::new(0x4FFF));
    }

    #[test_case]
    fn brk_out_of_range_returns_current() {
        use crate::memory::VirtAddr;
        let mut brk = Brk {
            brk_start: VirtAddr::new(0x1000),
            brk_current: VirtAddr::new(0x2000),
            brk_end: VirtAddr::new(0x5000),
        };
        // Below start
        assert_eq!(brk.brk(VirtAddr::new(0x0500)), VirtAddr::new(0x2000));
        // At end (exclusive boundary)
        assert_eq!(brk.brk(VirtAddr::new(0x5000)), VirtAddr::new(0x2000));
        // Above end
        assert_eq!(brk.brk(VirtAddr::new(0x9000)), VirtAddr::new(0x2000));
    }

    #[test_case]
    fn brk_empty() {
        use crate::memory::VirtAddr;
        let mut brk = Brk::empty();
        assert_eq!(brk.brk(VirtAddr::zero()), VirtAddr::zero());
        // brk_end is 1, so 0 is within [0, 1)
        assert_eq!(brk.brk(VirtAddr::new(0x1000)), VirtAddr::zero());
    }
}
