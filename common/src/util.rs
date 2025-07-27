use core::fmt::Display;

pub const fn align_up(value: usize, alignment: usize) -> usize {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + alignment - remainder
    }
}

pub fn align_down_ptr<T>(ptr: *const T, alignment: usize) -> *const T {
    ptr.mask(!(alignment - 1))
}

pub fn align_down(value: usize, alignment: usize) -> usize {
    value & !(alignment - 1)
}

pub struct PrintMemorySizeHumanFriendly(pub usize);

impl Display for PrintMemorySizeHumanFriendly {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut size = self.0 as f64;
        for format in ["", "KiB", "MiB", "GiB"] {
            if size < 1024.0 {
                return write!(f, "{size:.2} {format}");
            }
            size /= 1024.0;
        }
        write!(f, "{size:.2} TiB")
    }
}

pub fn copy_slice<T: Copy>(src: &[T], dst: &mut [T]) {
    assert!(dst.len() >= src.len());
    dst[..src.len()].copy_from_slice(src);
}
