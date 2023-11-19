use core::cell::RefCell;

use alloc::{collections::VecDeque, rc::Rc};
use common::mutex::Mutex;

use super::process::{Process, ProcessState, PID};

static PROCESSES: Mutex<VecDeque<Rc<RefCell<Process>>>> = Mutex::new(VecDeque::new());

pub fn add_process(process: Process) {
    PROCESSES.lock().push_front(Rc::new(RefCell::new(process)));
}

pub fn next_runnable() -> Option<Rc<RefCell<Process>>> {
    let mut processes = PROCESSES.lock();
    let mut index_to_remove = None;

    for (index, process) in processes.iter().enumerate() {
        if process.borrow().get_state() == ProcessState::Runnable {
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

pub fn enqueue(process: Rc<RefCell<Process>>) {
    PROCESSES.lock().push_back(process);
}

pub fn does_pid_exits(pid: PID) -> bool {
    PROCESSES
        .lock()
        .iter()
        .any(|process| process.borrow().get_pid() == pid)
}

pub fn notify_died(pid: PID) {
    let processes = PROCESSES.lock();
    for process in processes.iter() {
        if process.borrow().get_state() == ProcessState::WaitingFor(pid) {
            process.borrow_mut().set_state(ProcessState::Runnable);
        }
    }
}
