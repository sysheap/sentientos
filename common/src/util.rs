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
