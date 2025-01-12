use crate::{cpu::Cpu, debug, device_tree, sbi};
use common::{big_endian::BigEndian, runtime_initialized::RuntimeInitializedData};
use core::arch::asm;

pub const CLINT_BASE: usize = 0x2000000;
pub const CLINT_SIZE: usize = 0x10000;

static CLOCKS_PER_SEC: RuntimeInitializedData<u64> = RuntimeInitializedData::new();

pub fn init() {
    let clocks_per_sec = device_tree::THE
        .root_node()
        .find_node("cpus")
        .expect("There must be a cpus node")
        .get_property("timebase-frequency")
        .expect("There must be a timebase-frequency")
        .consume_sized_type::<BigEndian<u32>>()
        .expect("The value must be u32")
        .get() as u64;
    CLOCKS_PER_SEC.initialize(clocks_per_sec);
}

#[no_mangle]
pub extern "C" fn set_timer(milliseconds: u64) {
    debug!("enabling timer {milliseconds} ms");
    let current = get_current_clocks();
    assert_eq!(*CLOCKS_PER_SEC / 1000, 10_000);
    let next = current + ((*CLOCKS_PER_SEC / 1000) * milliseconds);
    sbi::extensions::timer_extension::sbi_set_timer(next).assert_success();
    Cpu::enable_timer_interrupt();
}

fn get_current_clocks() -> u64 {
    let current: u64;
    unsafe {
        asm!("rdtime {current}", current = out(reg)current);
    };
    current
}
