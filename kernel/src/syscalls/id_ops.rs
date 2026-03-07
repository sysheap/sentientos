use core::ffi::{c_int, c_uint};
use headers::{
    errno::Errno,
    syscall_types::{FUTEX_PRIVATE_FLAG, FUTEX_WAIT, FUTEX_WAKE},
};

use crate::processes::{
    futex::{self, FutexWait},
    process_table,
};
use common::pid::Tid;

use super::linux::LinuxSyscallHandler;

impl LinuxSyscallHandler {
    pub(super) fn do_getpgid(&self, pid: c_int) -> Result<isize, Errno> {
        if pid == 0 {
            let pgid = self.current_process.with_lock(|p| p.pgid());
            return Ok(pgid.as_isize());
        }
        let target = Tid::try_from_i32(pid).ok_or(Errno::ESRCH)?;
        let pgid = process_table::THE
            .lock()
            .get_pgid_of(target)
            .ok_or(Errno::ESRCH)?;
        Ok(pgid.as_isize())
    }

    pub(super) fn do_getsid(&self, pid: c_int) -> Result<isize, Errno> {
        if pid == 0 {
            let sid = self.current_process.with_lock(|p| p.sid());
            return Ok(sid.as_isize());
        }
        let target = Tid::try_from_i32(pid).ok_or(Errno::ESRCH)?;
        let sid = process_table::THE
            .lock()
            .get_sid_of(target)
            .ok_or(Errno::ESRCH)?;
        Ok(sid.as_isize())
    }

    pub(super) fn do_setpgid(&self, pid: c_int, pgid: c_int) -> Result<isize, Errno> {
        let my_main_tid = self.current_process.with_lock(|p| p.main_tid());
        let target_tid = if pid == 0 {
            my_main_tid
        } else {
            Tid::try_from_i32(pid).ok_or(Errno::EINVAL)?
        };
        let new_pgid = if pgid == 0 {
            target_tid
        } else {
            Tid::try_from_i32(pgid).ok_or(Errno::EINVAL)?
        };

        if target_tid != my_main_tid {
            let is_child = process_table::THE
                .lock()
                .is_child_of(my_main_tid, target_tid);
            if !is_child {
                return Err(Errno::ESRCH);
            }
        }

        if !process_table::THE.lock().set_pgid_of(target_tid, new_pgid) {
            return Err(Errno::ESRCH);
        }
        Ok(0)
    }

    pub(super) async fn do_futex(
        &self,
        uaddr: usize,
        op: c_int,
        val: c_uint,
    ) -> Result<isize, Errno> {
        let cmd = op & !(FUTEX_PRIVATE_FLAG as c_int);
        let main_tid = self.current_process.with_lock(|p| p.main_tid());
        match cmd.cast_unsigned() {
            FUTEX_WAIT => {
                let result =
                    FutexWait::new(self.current_process.clone(), uaddr, val, main_tid).await;
                Ok(result as isize)
            }
            FUTEX_WAKE => {
                let result = futex::futex_wake(main_tid, uaddr, val);
                Ok(result as isize)
            }
            _ => Err(Errno::ENOSYS),
        }
    }
}
