#[allow(dead_code)]
pub fn assert_unreachable(message: &str) -> ! {
    panic!("Unreachable: {}", message);
}

macro_rules! static_assert_size {
    ($type: ty, $size: expr) => {
        const _: [(); $size] = [(); core::mem::size_of::<$type>()];
    };
}

pub(crate) use static_assert_size;
