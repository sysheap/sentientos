use alloc::{sync::Arc, task::Wake};
use core::task::Waker;

pub struct TaskWaker;

impl TaskWaker {
    pub fn new() -> Waker {
        let task_waker = Arc::new(TaskWaker);
        task_waker.into()
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        todo!()
    }
}
