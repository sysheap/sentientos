use headers::syscall_types::{
    SIGABRT, SIGALRM, SIGBUS, SIGCHLD, SIGCONT, SIGFPE, SIGHUP, SIGILL, SIGINT, SIGIO, SIGKILL,
    SIGPIPE, SIGPROF, SIGPWR, SIGQUIT, SIGSEGV, SIGSTKFLT, SIGSTOP, SIGSYS, SIGTERM, SIGTRAP,
    SIGTSTP, SIGTTIN, SIGTTOU, SIGURG, SIGUSR1, SIGUSR2, SIGVTALRM, SIGWINCH, SIGXCPU, SIGXFSZ,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Exited(u8),
    Signaled(u8),
}

impl ExitStatus {
    pub fn to_wstatus(self) -> i32 {
        match self {
            ExitStatus::Exited(code) => i32::from(code) << 8,
            ExitStatus::Signaled(sig) => i32::from(sig) & 0x7f,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PendingSignals(u64);

impl PendingSignals {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn raise(&mut self, sig: u32) {
        assert!((1..=31).contains(&sig));
        self.0 |= 1u64 << sig;
    }

    pub fn clear(&mut self, sig: u32) {
        assert!((1..=31).contains(&sig));
        self.0 &= !(1u64 << sig);
    }

    pub fn first_unblocked(&self, mask: u64) -> Option<u32> {
        let deliverable = self.0 & !mask;
        if deliverable == 0 {
            return None;
        }
        Some(deliverable.trailing_zeros())
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultAction {
    Terminate,
    Ignore,
    Stop,
    Continue,
}

pub fn default_action(sig: u32) -> DefaultAction {
    match sig {
        SIGHUP | SIGINT | SIGQUIT | SIGILL | SIGTRAP | SIGABRT | SIGBUS | SIGFPE | SIGKILL
        | SIGUSR1 | SIGSEGV | SIGUSR2 | SIGPIPE | SIGALRM | SIGTERM | SIGSTKFLT | SIGXCPU
        | SIGXFSZ | SIGVTALRM | SIGPROF | SIGIO | SIGPWR | SIGSYS => DefaultAction::Terminate,
        SIGCHLD | SIGURG | SIGWINCH => DefaultAction::Ignore,
        SIGSTOP | SIGTSTP | SIGTTIN | SIGTTOU => DefaultAction::Stop,
        SIGCONT => DefaultAction::Continue,
        _ => DefaultAction::Terminate,
    }
}
