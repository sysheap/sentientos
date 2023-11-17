#[allow(dead_code)]
pub fn assert_unreachable(message: &str) -> ! {
    panic!("Unreachable: {}", message);
}
