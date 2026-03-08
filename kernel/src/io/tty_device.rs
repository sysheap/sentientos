use alloc::{collections::VecDeque, sync::Arc, vec::Vec};
use common::pid::Tid;
use core::{
    cmp::min,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use headers::syscall_types::{
    CLOCAL, CREAD, CS8, ECHO, ECHOE, ICANON, ICRNL, ISIG, NCCS, ONLCR, OPOST, VEOF, VERASE, VINTR,
    VKILL, VMIN, VQUIT, VSUSP, VTIME, termios,
};

use crate::klibc::{Spinlock, array_vec::ArrayVec, runtime_initialized::RuntimeInitializedData};

pub static CONSOLE_TTY: RuntimeInitializedData<TtyDevice> = RuntimeInitializedData::new();

pub fn console_tty() -> &'static TtyDevice {
    &CONSOLE_TTY
}

pub type TtyDevice = Arc<Spinlock<TtyDeviceInner>>;

pub struct InputResult {
    pub action: InputAction,
    pub echo: ArrayVec<u8, 192>,
}

pub enum InputAction {
    Consumed,
    Signal(u32),
}

pub struct TtyDeviceInner {
    settings: termios,
    line_buf: VecDeque<u8>,
    input_buf: VecDeque<u8>,
    wakeup_queue: Vec<Waker>,
    fg_pgid: Tid,
    eof_pending: bool,
}

impl TtyDeviceInner {
    pub fn new() -> Self {
        let mut c_cc = [0u8; NCCS as usize];
        c_cc[VINTR as usize] = 3; // Ctrl-C
        c_cc[VQUIT as usize] = 28; // Ctrl-backslash
        c_cc[VERASE as usize] = 127; // DEL
        c_cc[VKILL as usize] = 21; // Ctrl-U
        c_cc[VEOF as usize] = 4; // Ctrl-D
        c_cc[VSUSP as usize] = 26; // Ctrl-Z
        c_cc[VMIN as usize] = 1;
        c_cc[VTIME as usize] = 0;

        Self {
            settings: termios {
                c_iflag: ICRNL,
                c_oflag: OPOST | ONLCR,
                c_cflag: CS8 | CREAD | CLOCAL,
                c_lflag: ISIG | ICANON | ECHO | ECHOE,
                c_line: 0,
                c_cc,
            },
            line_buf: VecDeque::new(),
            input_buf: VecDeque::new(),
            wakeup_queue: Vec::new(),
            fg_pgid: Tid::new(1), // init process group by default
            eof_pending: false,
        }
    }

    pub fn get_termios(&self) -> termios {
        self.settings
    }

    pub fn set_termios(&mut self, new: termios) {
        self.settings = new;
    }

    pub fn fg_pgid(&self) -> Tid {
        self.fg_pgid
    }

    pub fn set_fg_pgid(&mut self, pgid: Tid) {
        self.fg_pgid = pgid;
    }

    fn has_lflag(&self, flag: u32) -> bool {
        self.settings.c_lflag & flag != 0
    }

    fn has_iflag(&self, flag: u32) -> bool {
        self.settings.c_iflag & flag != 0
    }

    fn has_oflag(&self, flag: u32) -> bool {
        self.settings.c_oflag & flag != 0
    }

    fn echo_newline(&self, echo: &mut ArrayVec<u8, 192>) {
        if self.has_oflag(OPOST) && self.has_oflag(ONLCR) {
            let _ = echo.push(b'\r');
        }
        let _ = echo.push(b'\n');
    }

    pub fn process_output(&self, data: &[u8]) -> Vec<u8> {
        if !(self.has_oflag(OPOST) && self.has_oflag(ONLCR)) {
            return data.to_vec();
        }
        let mut out = Vec::with_capacity(data.len());
        for &b in data {
            if b == b'\n' {
                out.push(b'\r');
            }
            out.push(b);
        }
        out
    }

    pub fn process_input_byte(&mut self, mut byte: u8) -> InputResult {
        let mut echo = ArrayVec::new();

        if self.has_iflag(ICRNL) && byte == b'\r' {
            byte = b'\n';
        }

        if self.has_lflag(ISIG) {
            if byte == self.settings.c_cc[VINTR as usize] {
                if self.has_lflag(ECHO) {
                    let _ = echo.push(b'^');
                    let _ = echo.push(b'C');
                    self.echo_newline(&mut echo);
                }
                self.line_buf.clear();
                return InputResult {
                    action: InputAction::Signal(headers::syscall_types::SIGINT),
                    echo,
                };
            }
            if byte == self.settings.c_cc[VSUSP as usize] {
                if self.has_lflag(ECHO) {
                    let _ = echo.push(b'^');
                    let _ = echo.push(b'Z');
                    self.echo_newline(&mut echo);
                }
                self.line_buf.clear();
                return InputResult {
                    action: InputAction::Signal(headers::syscall_types::SIGTSTP),
                    echo,
                };
            }
            if byte == self.settings.c_cc[VQUIT as usize] {
                if self.has_lflag(ECHO) {
                    let _ = echo.push(b'^');
                    let _ = echo.push(b'\\');
                    self.echo_newline(&mut echo);
                }
                self.line_buf.clear();
                return InputResult {
                    action: InputAction::Signal(headers::syscall_types::SIGQUIT),
                    echo,
                };
            }
        }

        if self.has_lflag(ICANON) {
            if byte == b'\n' {
                self.line_buf.push_back(byte);
                if self.has_lflag(ECHO) {
                    self.echo_newline(&mut echo);
                }
                self.flush_line_buf();
            } else if byte == self.settings.c_cc[VERASE as usize] {
                if self.line_buf.pop_back().is_some() && self.has_lflag(ECHOE) {
                    let _ = echo.push(b'\x08');
                    let _ = echo.push(b' ');
                    let _ = echo.push(b'\x08');
                }
            } else if byte == self.settings.c_cc[VEOF as usize] {
                if self.line_buf.is_empty() {
                    self.eof_pending = true;
                    self.wake_all();
                } else {
                    self.flush_line_buf();
                }
            } else if byte == self.settings.c_cc[VKILL as usize] {
                if self.has_lflag(ECHOE) {
                    for _ in 0..self.line_buf.len() {
                        let _ = echo.push(b'\x08');
                        let _ = echo.push(b' ');
                        let _ = echo.push(b'\x08');
                    }
                }
                self.line_buf.clear();
            } else {
                self.line_buf.push_back(byte);
                if self.has_lflag(ECHO) {
                    let _ = echo.push(byte);
                }
            }
        } else {
            self.push_input(byte);
            if self.has_lflag(ECHO) {
                let _ = echo.push(byte);
            }
        }

        InputResult {
            action: InputAction::Consumed,
            echo,
        }
    }

    fn flush_line_buf(&mut self) {
        self.input_buf.extend(self.line_buf.drain(..));
        self.eof_pending = false;
        self.wake_all();
    }

    fn push_input(&mut self, byte: u8) {
        self.input_buf.push_back(byte);
        self.wake_all();
    }

    fn wake_all(&mut self) {
        while let Some(waker) = self.wakeup_queue.pop() {
            waker.wake();
        }
    }

    pub fn get_input(&mut self, count: usize) -> Vec<u8> {
        let actual_count = min(self.input_buf.len(), count);
        self.input_buf.drain(..actual_count).collect()
    }

    pub fn is_input_empty(&self) -> bool {
        self.input_buf.is_empty()
    }

    fn register_wakeup(&mut self, waker: Waker) {
        self.wakeup_queue.push(waker);
    }

    fn vmin(&self) -> usize {
        self.settings.c_cc[VMIN as usize] as usize
    }
}

pub struct ReadTty {
    device: TtyDevice,
    max_count: usize,
    is_registered: bool,
}

impl ReadTty {
    pub fn new(device: TtyDevice, max_count: usize) -> Self {
        Self {
            device,
            max_count,
            is_registered: false,
        }
    }
}

impl Future for ReadTty {
    type Output = Vec<u8>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let mut dev = this.device.lock();
        let is_canonical = dev.has_lflag(ICANON);
        let vmin = dev.vmin();

        if !dev.is_input_empty() {
            let min_needed = if is_canonical { 1 } else { vmin.max(1) };
            if dev.input_buf.len() >= min_needed || dev.input_buf.len() >= this.max_count {
                return Poll::Ready(dev.get_input(this.max_count));
            }
        }

        if is_canonical && dev.eof_pending {
            dev.eof_pending = false;
            return Poll::Ready(Vec::new());
        }

        if !is_canonical && vmin == 0 {
            return Poll::Ready(dev.get_input(this.max_count));
        }

        if !this.is_registered {
            let waker = cx.waker().clone();
            dev.register_wakeup(waker);
            this.is_registered = true;
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn echo_regular_chars() {
        let mut dev = TtyDeviceInner::new();
        let r = dev.process_input_byte(b'a');
        assert!(matches!(r.action, InputAction::Consumed));
        assert_eq!(&*r.echo, b"a");
        assert!(dev.is_input_empty());
    }

    #[test_case]
    fn newline_flushes_line() {
        let mut dev = TtyDeviceInner::new();
        dev.process_input_byte(b'h');
        dev.process_input_byte(b'i');
        let r = dev.process_input_byte(b'\n');
        assert_eq!(&*r.echo, b"\r\n");
        assert_eq!(dev.get_input(1024), b"hi\n");
    }

    #[test_case]
    fn cr_mapped_to_nl() {
        let mut dev = TtyDeviceInner::new();
        dev.process_input_byte(b'x');
        let r = dev.process_input_byte(b'\r');
        assert_eq!(&*r.echo, b"\r\n");
        assert_eq!(dev.get_input(1024), b"x\n");
    }

    #[test_case]
    fn backspace_erases() {
        let mut dev = TtyDeviceInner::new();
        dev.process_input_byte(b'a');
        dev.process_input_byte(b'b');
        let r = dev.process_input_byte(127);
        assert_eq!(&*r.echo, b"\x08 \x08");
        dev.process_input_byte(b'\n');
        assert_eq!(dev.get_input(1024), b"a\n");
    }

    #[test_case]
    fn backspace_on_empty_does_nothing() {
        let mut dev = TtyDeviceInner::new();
        let r = dev.process_input_byte(127);
        assert!(r.echo.is_empty());
    }

    #[test_case]
    fn ctrl_c_generates_sigint() {
        let mut dev = TtyDeviceInner::new();
        dev.process_input_byte(b'x');
        let r = dev.process_input_byte(3);
        assert!(
            matches!(r.action, InputAction::Signal(sig) if sig == headers::syscall_types::SIGINT)
        );
        dev.process_input_byte(b'\n');
        assert_eq!(dev.get_input(1024), b"\n");
    }

    #[test_case]
    fn ctrl_z_generates_sigtstp() {
        let mut dev = TtyDeviceInner::new();
        let r = dev.process_input_byte(26);
        assert!(
            matches!(r.action, InputAction::Signal(sig) if sig == headers::syscall_types::SIGTSTP)
        );
        assert_eq!(&*r.echo, b"^Z\r\n");
    }

    #[test_case]
    fn ctrl_backslash_generates_sigquit() {
        let mut dev = TtyDeviceInner::new();
        let r = dev.process_input_byte(28);
        assert!(
            matches!(r.action, InputAction::Signal(sig) if sig == headers::syscall_types::SIGQUIT)
        );
        assert_eq!(&*r.echo, b"^\\\r\n");
    }

    #[test_case]
    fn ctrl_d_on_empty_line_sets_eof() {
        let mut dev = TtyDeviceInner::new();
        let r = dev.process_input_byte(4); // Ctrl-D
        assert!(matches!(r.action, InputAction::Consumed));
        assert!(dev.eof_pending);
        assert!(dev.is_input_empty());
    }

    #[test_case]
    fn ctrl_d_with_data_flushes_without_eof() {
        let mut dev = TtyDeviceInner::new();
        dev.process_input_byte(b'a');
        dev.process_input_byte(b'b');
        let r = dev.process_input_byte(4); // Ctrl-D
        assert!(matches!(r.action, InputAction::Consumed));
        assert!(!dev.eof_pending);
        assert_eq!(dev.get_input(1024), b"ab");
    }

    #[test_case]
    fn new_line_clears_stale_eof_pending() {
        let mut dev = TtyDeviceInner::new();
        dev.process_input_byte(4); // Ctrl-D on empty line
        assert!(dev.eof_pending);
        dev.process_input_byte(b'x');
        dev.process_input_byte(b'\n');
        assert!(!dev.eof_pending);
        assert_eq!(dev.get_input(1024), b"x\n");
    }

    #[test_case]
    fn onlcr_output_processing() {
        let dev = TtyDeviceInner::new();
        assert_eq!(dev.process_output(b"hello\nworld\n"), b"hello\r\nworld\r\n");
    }

    #[test_case]
    fn no_onlcr_when_disabled() {
        let mut dev = TtyDeviceInner::new();
        dev.settings.c_oflag = 0;
        assert_eq!(dev.process_output(b"hello\nworld\n"), b"hello\nworld\n");
    }
}
