use crate::{
    cpu::Cpu, debug, device_tree, klibc::btreemap::SplitOffLowerThan,
    processes::thread::ThreadWeakRef, sbi,
};
use alloc::{collections::BTreeMap, vec::Vec};
use common::{big_endian::BigEndian, mutex::Mutex, runtime_initialized::RuntimeInitializedData};
use core::arch::asm;
use headers::{errno::Errno, syscall_types::timespec};

pub const CLINT_BASE: usize = 0x2000000;
pub const CLINT_SIZE: usize = 0x10000;

static CLOCKS_PER_NANO: RuntimeInitializedData<u64> = RuntimeInitializedData::new();

type WakeupClockTime = u64;

static WAKEUP_QUEUE: Mutex<BTreeMap<WakeupClockTime, ThreadWeakRef>> = Mutex::new(BTreeMap::new());

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
    CLOCKS_PER_NANO.initialize(clocks_per_sec / 1000 / 1000);
}

pub fn register_wakeup(duration: &timespec, thread: ThreadWeakRef) -> Result<(), Errno> {
    let clocks_per_nano = *CLOCKS_PER_NANO;
    let clocks_per_second = clocks_per_nano * 1000 * 1000;
    let clocks = u64::try_from(duration.tv_sec)? * clocks_per_second
        + u64::try_from(duration.tv_nsec)? * clocks_per_nano;
    let wakeup_time = get_current_clocks() + clocks;
    WAKEUP_QUEUE.lock().insert(wakeup_time, thread);
    Ok(())
}

pub fn return_threads_to_wakeup() -> Vec<ThreadWeakRef> {
    let current = get_current_clocks();
    let mut lg = WAKEUP_QUEUE.lock();
    let threads = lg.split_off_lower_than(&current);
    threads.into_values().collect()
}

pub fn disable_timer() {
    debug!("disabling timer");
    Cpu::disable_timer_interrupt();
}

pub fn set_timer(milliseconds: u64) {
    debug!("enabling timer {milliseconds} ms");
    let current = get_current_clocks();
    let next = current.wrapping_add(*CLOCKS_PER_NANO * 1000 * milliseconds);
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
