use common::pid::Tid;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use headers::errno::Errno;

use super::process_table;

pub struct WaitChild {
    parent_main_tid: Tid,
    pid: i32,
    wnohang: bool,
}

impl WaitChild {
    pub fn new(parent_main_tid: Tid, pid: i32, wnohang: bool) -> Self {
        Self {
            parent_main_tid,
            pid,
            wnohang,
        }
    }
}

impl Future for WaitChild {
    type Output = Result<(Tid, i32), Errno>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        process_table::THE.with_lock(|mut pt| {
            if let Some((tid, status)) = pt.take_zombie(self.parent_main_tid, self.pid) {
                return Poll::Ready(Ok((tid, status)));
            }

            if !pt.has_any_child_of(self.parent_main_tid) {
                return Poll::Ready(Err(Errno::ECHILD));
            }

            if self.wnohang {
                return Poll::Ready(Ok((Tid(0), 0)));
            }

            pt.register_wait_waker(cx.waker().clone());
            Poll::Pending
        })
    }
}
