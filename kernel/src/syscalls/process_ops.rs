use alloc::{collections::BTreeMap, string::String, sync::Arc};
use core::ffi::{c_int, c_ulong};
use headers::{
    errno::Errno,
    syscall_types::{
        CLONE_CHILD_CLEARTID, CLONE_PARENT_SETTID, CLONE_SETTLS, CLONE_VFORK, CLONE_VM, SIGCHLD,
    },
};

use crate::{
    cpu::Cpu,
    memory::VirtAddr,
    processes::{
        brk::Brk,
        process::Process,
        process_table,
        thread::{Thread, VforkState, VforkWait, get_next_tid},
        wait_child::{WaitChild, WaitPid},
    },
    syscalls::linux_validator::LinuxUserspaceArg,
};
use common::{pid::Tid, syscalls::trap_frame::Register};

use super::linux::LinuxSyscallHandler;

impl LinuxSyscallHandler {
    pub(super) async fn clone_vfork(
        &mut self,
        flags: c_ulong,
        stack: usize,
    ) -> Result<isize, Errno> {
        let expected = c_ulong::from(CLONE_VM | CLONE_VFORK | SIGCHLD);
        assert!(
            flags == expected,
            "clone_vfork: unsupported flags {flags:#x}, expected {expected:#x}"
        );

        let parent_regs = Cpu::read_trap_frame();
        let parent_pc = arch::cpu::read_sepc();

        let parent_process = self.current_process.clone();
        let (parent_main_tid, child_name, parent_pgid, parent_sid) =
            parent_process.with_lock(|p| {
                (
                    p.main_tid(),
                    Arc::new(String::from(p.get_name())),
                    p.pgid(),
                    p.sid(),
                )
            });

        let vfork_state = VforkState::new();
        let child_tid = get_next_tid();

        let child_page_table =
            crate::memory::page_tables::RootPageTableHolder::new_with_kernel_mapping(false);
        let child_process = Arc::new(crate::klibc::Spinlock::new(Process::new(
            child_name.clone(),
            child_page_table,
            BTreeMap::new(),
            Brk::empty(),
            child_tid,
            parent_pgid,
            parent_sid,
        )));
        child_process
            .lock()
            .set_vfork_parent(parent_process.clone());

        let (parent_fd_table, parent_cwd) =
            parent_process.with_lock(|p| (p.fd_table().clone(), String::from(p.cwd())));
        {
            let mut child = child_process.lock();
            child.set_fd_table(parent_fd_table);
            child.set_cwd(parent_cwd);
        }

        let mut child_regs = parent_regs;
        child_regs[Register::a0] = 0;
        if stack != 0 {
            child_regs[Register::sp] = stack;
        }

        let child_thread = Thread::new(
            child_tid,
            child_name,
            child_regs,
            VirtAddr::new(parent_pc + 4),
            false,
            child_process.clone(),
            parent_main_tid,
        );

        child_thread.lock().set_vfork_state(vfork_state.clone());

        child_process
            .lock()
            .add_thread(child_tid, Arc::downgrade(&child_thread));
        process_table::THE.lock().add_thread(child_thread);

        VforkWait::new(vfork_state).await;

        Ok(child_tid.as_isize())
    }

    pub(super) fn clone_thread(
        &mut self,
        flags: c_ulong,
        stack: usize,
        ptid: LinuxUserspaceArg<Option<*mut c_int>>,
        tls: c_ulong,
        ctid: LinuxUserspaceArg<Option<*mut c_int>>,
    ) -> Result<isize, Errno> {
        let parent_regs = Cpu::read_trap_frame();
        let parent_pc = arch::cpu::read_sepc();

        let parent_process = self.current_process.clone();
        let (parent_main_tid, child_name) =
            parent_process.with_lock(|p| (p.main_tid(), Arc::new(String::from(p.get_name()))));

        let child_tid = get_next_tid();

        let mut child_regs = parent_regs;
        child_regs[Register::a0] = 0;
        if stack != 0 {
            child_regs[Register::sp] = stack;
        }
        if (flags & c_ulong::from(CLONE_SETTLS)) != 0 {
            child_regs[Register::tp] = usize::try_from(tls).expect("tls fits in usize");
        }

        let child_thread = Thread::new(
            child_tid,
            child_name,
            child_regs,
            VirtAddr::new(parent_pc + 4),
            false,
            parent_process.clone(),
            parent_main_tid,
        );

        if (flags & c_ulong::from(CLONE_CHILD_CLEARTID)) != 0 {
            child_thread.lock().set_clear_child_tid((&ctid).into());
        }

        parent_process.with_lock(|mut p| {
            p.add_thread(child_tid, Arc::downgrade(&child_thread));
        });

        if (flags & c_ulong::from(CLONE_PARENT_SETTID)) != 0 {
            ptid.write_if_not_none(
                c_int::try_from(child_tid.as_isize()).expect("tid fits in c_int"),
            )?;
        }

        process_table::THE.lock().add_thread(child_thread);

        Ok(child_tid.as_isize())
    }

    pub(super) async fn do_wait4(
        &self,
        pid: c_int,
        status: LinuxUserspaceArg<Option<*mut c_int>>,
        options: c_int,
    ) -> Result<isize, Errno> {
        let wnohang = (options & headers::syscall_types::WNOHANG as c_int) != 0;
        assert!(
            options & !(headers::syscall_types::WNOHANG as c_int) == 0,
            "wait4: unsupported options {options:#x}"
        );

        let parent_main_tid = self.current_thread.lock().get_tid();
        let target = if pid > 0 {
            WaitPid::Specific(Tid::try_from_i32(pid).expect("pid is positive"))
        } else if pid == -1 {
            WaitPid::Any
        } else if pid == 0 {
            let own_pgid = self.current_process.with_lock(|p| p.pgid());
            WaitPid::Pgid(own_pgid)
        } else {
            // pid < -1: wait for any child whose pgid == abs(pid)
            let abs_pid = pid.checked_neg().ok_or(Errno::EINVAL)?;
            WaitPid::Pgid(Tid::try_from_i32(abs_pid).expect("abs(pid) is positive"))
        };
        let (child_tid, wait_status) = WaitChild::new(parent_main_tid, target, wnohang).await?;

        status.write_if_not_none(wait_status)?;

        Ok(child_tid.as_isize())
    }
}
