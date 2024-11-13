#[allow(non_snake_case)]
pub fn KiB(value: usize) -> usize {
    value * 1024
}

#[allow(non_snake_case)]
pub fn MiB(value: usize) -> usize {
    KiB(value) * 1024
}

#[allow(non_snake_case)]
pub fn GiB(value: usize) -> usize {
    MiB(value) * 1024
}
