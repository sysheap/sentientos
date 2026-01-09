use core::{
    any::type_name,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

use alloc::boxed::Box;

#[derive(Debug)]
pub struct TaskId(pub u64);

impl TaskId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let next = COUNTER.fetch_add(1, Ordering::Relaxed);
        assert!(next != u64::MAX, "Max number of tasks reached");
        Self(next)
    }
}

pub struct Task<Output = ()> {
    task_id: TaskId,
    future: Pin<Box<dyn Future<Output = Output> + Send + 'static>>,
}

impl<Output> core::fmt::Debug for Task<Output> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Task")
            .field("task_id", &self.task_id)
            .field_with("future", |f| {
                write!(f, "Future<Output = {}>", type_name::<Output>())
            })
            .finish()
    }
}

impl<Output> Task<Output> {
    pub fn new(future: impl Future<Output = Output> + Send + 'static) -> Self {
        Self {
            task_id: TaskId::new(),
            future: Box::pin(future),
        }
    }
    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Output> {
        self.future.as_mut().poll(cx)
    }
}
