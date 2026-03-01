use crate::{
    cpu::Cpu,
    println,
    syscalls::{
        linux::{LinuxSyscallHandler, LinuxSyscalls, SYSCALL_METADATA},
        trace_config::TRACED_PROCESSES,
    },
};
use common::syscalls::trap_frame::{Register, TrapFrame};
use core::ffi::{c_int, c_uint, c_ulong};
use headers::errno::Errno;

#[derive(Clone, Copy)]
pub enum ArgFormat {
    SignedDec,
    Hex,
    Pointer,
}

pub struct SyscallMetadata {
    pub name: &'static str,
    pub args: &'static [(&'static str, ArgFormat)],
}

pub trait SyscallArgFormat {
    const FORMAT: ArgFormat;
}

impl SyscallArgFormat for c_int {
    const FORMAT: ArgFormat = ArgFormat::SignedDec;
}
impl SyscallArgFormat for c_uint {
    const FORMAT: ArgFormat = ArgFormat::Hex;
}
impl SyscallArgFormat for c_ulong {
    const FORMAT: ArgFormat = ArgFormat::Hex;
}
impl SyscallArgFormat for usize {
    const FORMAT: ArgFormat = ArgFormat::Hex;
}
impl SyscallArgFormat for isize {
    const FORMAT: ArgFormat = ArgFormat::SignedDec;
}
impl<T> SyscallArgFormat for *const T {
    const FORMAT: ArgFormat = ArgFormat::Pointer;
}
impl<T> SyscallArgFormat for *mut T {
    const FORMAT: ArgFormat = ArgFormat::Pointer;
}
impl<T> SyscallArgFormat for Option<*const T> {
    const FORMAT: ArgFormat = ArgFormat::Pointer;
}
impl<T> SyscallArgFormat for Option<*mut T> {
    const FORMAT: ArgFormat = ArgFormat::Pointer;
}

fn should_trace() -> bool {
    if TRACED_PROCESSES.is_empty() {
        return false;
    }
    Cpu::with_current_process(|p| TRACED_PROCESSES.contains(&p.get_name()))
}

fn find_metadata(nr: usize) -> Option<&'static SyscallMetadata> {
    SYSCALL_METADATA
        .iter()
        .find(|(n, _)| *n == nr)
        .map(|(_, m)| m)
}

fn format_arg(raw: usize, fmt: ArgFormat) -> alloc::string::String {
    match fmt {
        ArgFormat::SignedDec => alloc::format!("{}", raw as isize),
        ArgFormat::Hex => alloc::format!("{:#x}", raw),
        ArgFormat::Pointer if raw == 0 => alloc::string::String::from("NULL"),
        ArgFormat::Pointer => alloc::format!("{:#x}", raw),
    }
}

fn log_enter(trap_frame: &TrapFrame, tid: common::pid::Tid) {
    let nr = trap_frame[Register::a7];
    let args = [
        trap_frame[Register::a0],
        trap_frame[Register::a1],
        trap_frame[Register::a2],
        trap_frame[Register::a3],
        trap_frame[Register::a4],
        trap_frame[Register::a5],
    ];

    let Some(meta) = find_metadata(nr) else {
        println!("[SYSCALL ENTER] tid={tid} syscall_{nr}(...)");
        return;
    };

    let mut arg_strs = alloc::string::String::new();
    for (i, (name, fmt)) in meta.args.iter().enumerate() {
        if i > 0 {
            arg_strs.push_str(", ");
        }
        arg_strs.push_str(name);
        arg_strs.push_str(": ");
        arg_strs.push_str(&format_arg(args[i], *fmt));
    }

    println!("[SYSCALL ENTER] tid={tid} {}({arg_strs})", meta.name);
}

fn log_exit(trap_frame: &TrapFrame, tid: common::pid::Tid, result: &Result<isize, Errno>) {
    let nr = trap_frame[Register::a7];
    let name = find_metadata(nr).map(|m| m.name).unwrap_or("unknown");

    match result {
        Ok(val) => println!("[SYSCALL EXIT]  tid={tid} {name} = {val}"),
        Err(e) => println!(
            "[SYSCALL EXIT]  tid={tid} {name} = -{} ({e:?})",
            *e as isize
        ),
    }
}

pub async fn trace_syscall(
    trap_frame: &TrapFrame,
    handler: &mut LinuxSyscallHandler,
) -> Result<isize, Errno> {
    let tracing = should_trace();
    let tid = if tracing {
        let tid = Cpu::with_scheduler(|s| s.get_current_thread().lock().get_tid());
        log_enter(trap_frame, tid);
        Some(tid)
    } else {
        None
    };
    let result = handler.handle(trap_frame).await;
    if let Some(tid) = tid {
        log_exit(trap_frame, tid, &result);
    }
    result
}
