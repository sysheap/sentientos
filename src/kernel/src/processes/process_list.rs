use core::cell::RefCell;

use alloc::{collections::VecDeque, rc::Rc};
use common::mutex::Mutex;

use super::process::Process;

static PROCESSES: Mutex<VecDeque<Rc<RefCell<Process>>>> = Mutex::new(VecDeque::new());

pub fn add_process(process: Process) {
    PROCESSES.lock().push_front(Rc::new(RefCell::new(process)));
}

pub fn next_runnable() -> Option<Rc<RefCell<Process>>> {
    PROCESSES.lock().pop_front()
}

pub fn enqueue(process: Rc<RefCell<Process>>) {
    PROCESSES.lock().push_back(process);
}
