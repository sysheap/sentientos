use crate::{info, klibc::sizes::MiB, processes::scheduler};

pub mod backtrace;
mod eh_frame_parser;
pub mod symbols;
mod unwinder;

pub fn dump_current_state() {
    let allocated_size_heap = crate::memory::heap::allocated_size();
    info!(
        "Heap allocated: {:.2} MiB",
        allocated_size_heap as f64 / MiB(1) as f64
    );

    let total_heap_pages = crate::memory::total_heap_pages();
    let used_heap_pages = crate::memory::used_heap_pages();

    info!(
        "Page allocator {} / {} used",
        used_heap_pages, total_heap_pages
    );

    scheduler::THE.with_lock(|s| {
        s.dump();
        let current_process = s.get_current_process().lock();
        info!(
            "Current Process: PID={} NAME={} STATE={:?}",
            current_process.get_pid(),
            current_process.get_name(),
            current_process.get_state()
        );
    });
}
