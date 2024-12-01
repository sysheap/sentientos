use crate::{
    info,
    klibc::sizes::MiB,
    processes::{process_list, scheduler::get_current_process},
};

pub mod backtrace;
mod eh_frame_parser;
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

    process_list::dump();

    let current_process = get_current_process();
    if let Some(process) = current_process {
        let process = process.lock();
        info!("Current process:\n{:?}", *process);
    }
}
