use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    task::Wake,
};
use core::{
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Waker},
};

use crate::klibc::Spinlock;

use super::task::Task;

static TASKS: Spinlock<BTreeMap<usize, Task<()>>> = Spinlock::new(BTreeMap::new());
static READY_IDS: Spinlock<VecDeque<usize>> = Spinlock::new(VecDeque::new());
static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

pub fn spawn(future: impl Future<Output = ()> + Send + 'static) {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    TASKS.lock().insert(id, Task::new(future));
    READY_IDS.lock().push_back(id);
}

pub fn poll_ready_tasks() {
    loop {
        let id = match READY_IDS.lock().pop_front() {
            Some(id) => id,
            None => return,
        };
        let mut task = match TASKS.lock().remove(&id) {
            Some(task) => task,
            None => {
                // Task is being polled on another CPU. Re-queue so the
                // wakeup isn't lost; return to avoid spinning.
                READY_IDS.lock().push_back(id);
                return;
            }
        };
        let waker = Waker::from(Arc::new(KernelTaskWaker { task_id: id }));
        let mut cx = Context::from_waker(&waker);
        if task.poll(&mut cx).is_pending() {
            TASKS.lock().insert(id, task);
        }
    }
}

struct KernelTaskWaker {
    task_id: usize,
}

impl Wake for KernelTaskWaker {
    fn wake(self: Arc<Self>) {
        READY_IDS.lock().push_back(self.task_id);
    }
}
