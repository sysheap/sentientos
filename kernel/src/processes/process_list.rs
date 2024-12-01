use alloc::{collections::VecDeque, sync::Arc};
use common::mutex::Mutex;

use crate::info;

use super::process::{Pid, Process, ProcessState};

static PROCESSES: Mutex<VecDeque<Arc<Mutex<Process>>>> = Mutex::new(VecDeque::new());

pub fn add_process(process: Process) {
    PROCESSES.lock().push_front(Arc::new(Mutex::new(process)));
}

pub fn is_empty() -> bool {
    PROCESSES.lock().is_empty()
}

pub fn dump() {
    let processes = PROCESSES.lock();
    for process in &*processes {
        let process = process.lock();
        info!(
            "PID={} NAME={} STATE={:?}",
            process.get_pid(),
            process.get_name(),
            process.get_state()
        );
    }
}

pub fn next_runnable() -> Option<Arc<Mutex<Process>>> {
    let mut processes = PROCESSES.lock();
    let mut index_to_remove = None;

    for (index, process) in processes.iter().enumerate() {
        if process.lock().get_state() == ProcessState::Runnable {
            // Replace this condition with your property
            index_to_remove = Some(index);
            break;
        }
    }

    if let Some(index) = index_to_remove {
        processes.remove(index)
    } else {
        None
    }
}

pub fn enqueue(process: Arc<Mutex<Process>>) {
    PROCESSES.lock().push_back(process);
}

pub fn does_pid_exits(pid: Pid) -> bool {
    PROCESSES
        .lock()
        .iter()
        .any(|process| process.lock().get_pid() == pid)
}

pub fn notify_died(pid: Pid) {
    let processes = PROCESSES.lock();
    for process in processes.iter() {
        if process.lock().get_state() == ProcessState::WaitingFor(pid) {
            process.lock().set_state(ProcessState::Runnable);
        }
    }
}

pub fn notify_input() {
    let processes = PROCESSES.lock();
    for process in processes.iter() {
        let mut process = process.lock();
        if process.get_state() == ProcessState::WaitingForInput {
            process.set_state(ProcessState::Runnable);
        }
    }
}
