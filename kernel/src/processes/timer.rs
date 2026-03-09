use crate::{
    debug, device_tree,
    klibc::{
        Spinlock, big_endian::BigEndian, btreemap::SplitOffLowerThan,
        runtime_initialized::RuntimeInitializedData,
    },
};
use alloc::collections::BTreeMap;
use core::{
    pin::Pin,
    task::{Context, Poll, Waker},
};
use headers::{errno::Errno, syscall_types::timespec};

pub use arch::timer::{CLINT_BASE, CLINT_SIZE};

static CLOCKS_PER_NANO: RuntimeInitializedData<u64> = RuntimeInitializedData::new();

type WakeupClockTime = u64;

// Use Vec<Waker> to support multiple threads with the same wakeup_time.
// This prevents waker collision when multiple threads sleep for the same duration.
static WAKEUP_QUEUE: Spinlock<BTreeMap<WakeupClockTime, alloc::vec::Vec<Waker>>> =
    Spinlock::new(BTreeMap::new());

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

pub struct Sleep {
    wakeup_time: u64,
    registered: bool,
}

impl Sleep {
    fn new(wakeup_time: u64) -> Self {
        Self {
            wakeup_time,
            registered: false,
        }
    }
}

pub fn sleep(duration: &timespec) -> Result<Sleep, Errno> {
    let clocks_per_nano = *CLOCKS_PER_NANO;
    let clocks_per_second = clocks_per_nano * 1000 * 1000;
    let clocks = u64::try_from(duration.tv_sec)? * clocks_per_second
        + u64::try_from(duration.tv_nsec)? * clocks_per_nano;
    let wakeup_time = arch::timer::get_current_clocks() + clocks;
    Ok(Sleep::new(wakeup_time))
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if arch::timer::get_current_clocks() >= self.wakeup_time {
            return Poll::Ready(());
        }
        if !self.registered {
            let waker = cx.waker().clone();
            WAKEUP_QUEUE
                .lock()
                .entry(self.wakeup_time)
                .or_default()
                .push(waker);
            self.registered = true;
        }
        Poll::Pending
    }
}

pub fn wakeup_wakers() {
    let current = arch::timer::get_current_clocks();
    let mut lg = WAKEUP_QUEUE.lock();
    let threads = lg.split_off_lower_than(&current);
    for wakers in threads.into_values() {
        for waker in wakers {
            waker.wake();
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
pub fn current_time() -> timespec {
    let clocks = arch::timer::get_current_clocks();
    let clocks_per_nano = *CLOCKS_PER_NANO;
    let clocks_per_second = clocks_per_nano * 1000 * 1000;
    let secs = clocks / clocks_per_second;
    let remaining_clocks = clocks % clocks_per_second;
    let nsecs = remaining_clocks / clocks_per_nano;
    timespec {
        tv_sec: secs as i64,
        tv_nsec: nsecs as i64,
    }
}

pub fn set_timer(milliseconds: u64) {
    debug!("enabling timer {milliseconds} ms");
    let current = arch::timer::get_current_clocks();
    let next = current.wrapping_add(*CLOCKS_PER_NANO * 1000 * milliseconds);
    arch::sbi::extensions::timer_extension::sbi_set_timer(next).assert_success();
    arch::cpu::enable_timer_interrupt();
}
