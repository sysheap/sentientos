use alloc::collections::VecDeque;
use headers::syscall_types::{
    CLOCAL, CREAD, CS8, ECHO, ECHOE, ICANON, ICRNL, ISIG, NCCS, VEOF, VERASE, VINTR, VKILL, VQUIT,
    termios,
};

use crate::klibc::{Spinlock, array_vec::ArrayVec};

use super::stdin_buf::STDIN_BUFFER;

pub static TTY: Spinlock<TtyState> = Spinlock::new(TtyState::new());

pub struct InputResult {
    pub action: InputAction,
    pub echo: ArrayVec<u8, 192>,
}

pub enum InputAction {
    Consumed,
    Signal,
}

pub struct TtyState {
    settings: termios,
    line_buf: VecDeque<u8>,
}

impl TtyState {
    const fn new() -> Self {
        let mut c_cc = [0u8; NCCS as usize];
        c_cc[VINTR as usize] = 3; // Ctrl-C
        c_cc[VQUIT as usize] = 28; // Ctrl-backslash
        c_cc[VERASE as usize] = 127; // DEL
        c_cc[VKILL as usize] = 21; // Ctrl-U
        c_cc[VEOF as usize] = 4; // Ctrl-D

        Self {
            settings: termios {
                c_iflag: ICRNL,
                c_oflag: 0,
                c_cflag: CS8 | CREAD | CLOCAL,
                c_lflag: ISIG | ICANON | ECHO | ECHOE,
                c_line: 0,
                c_cc,
            },
            line_buf: VecDeque::new(),
        }
    }

    pub fn get_termios(&self) -> termios {
        self.settings
    }

    pub fn set_termios(&mut self, new: termios) {
        self.settings = new;
    }

    fn has_flag(&self, lflag: u32) -> bool {
        self.settings.c_lflag & lflag != 0
    }

    fn has_iflag(&self, iflag: u32) -> bool {
        self.settings.c_iflag & iflag != 0
    }

    pub fn process_input_byte(&mut self, mut byte: u8) -> InputResult {
        let mut echo = ArrayVec::new();

        // ICRNL: map CR → NL
        if self.has_iflag(ICRNL) && byte == b'\r' {
            byte = b'\n';
        }

        // ISIG: check for signal characters
        if self.has_flag(ISIG) && byte == self.settings.c_cc[VINTR as usize] {
            // Echo ^C and newline
            if self.has_flag(ECHO) {
                let _ = echo.push(b'^');
                let _ = echo.push(b'C');
                let _ = echo.push(b'\n');
            }
            self.line_buf.clear();
            return InputResult {
                action: InputAction::Signal,
                echo,
            };
        }

        if self.has_flag(ICANON) {
            if byte == b'\n' {
                self.line_buf.push_back(byte);
                if self.has_flag(ECHO) {
                    let _ = echo.push(b'\n');
                }
                let mut buf = STDIN_BUFFER.lock();
                for b in self.line_buf.drain(..) {
                    buf.push(b);
                }
            } else if byte == self.settings.c_cc[VERASE as usize] {
                if self.line_buf.pop_back().is_some() && self.has_flag(ECHOE) {
                    let _ = echo.push(b'\x08'); // backspace
                    let _ = echo.push(b' ');
                    let _ = echo.push(b'\x08');
                }
            } else if byte == self.settings.c_cc[VEOF as usize] {
                // Ctrl-D: flush line buffer without adding EOF byte
                if !self.line_buf.is_empty() {
                    let mut buf = STDIN_BUFFER.lock();
                    for b in self.line_buf.drain(..) {
                        buf.push(b);
                    }
                }
            } else if byte == self.settings.c_cc[VKILL as usize] {
                if self.has_flag(ECHOE) {
                    for _ in 0..self.line_buf.len() {
                        let _ = echo.push(b'\x08');
                        let _ = echo.push(b' ');
                        let _ = echo.push(b'\x08');
                    }
                }
                self.line_buf.clear();
            } else {
                self.line_buf.push_back(byte);
                if self.has_flag(ECHO) {
                    let _ = echo.push(byte);
                }
            }
        } else {
            // Raw mode
            STDIN_BUFFER.lock().push(byte);
            if self.has_flag(ECHO) {
                let _ = echo.push(byte);
            }
        }

        InputResult {
            action: InputAction::Consumed,
            echo,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain_stdin() -> alloc::vec::Vec<u8> {
        STDIN_BUFFER.lock().get(1024)
    }

    #[test_case]
    fn echo_regular_chars() {
        let mut tty = TtyState::new();
        let r = tty.process_input_byte(b'a');
        assert!(matches!(r.action, InputAction::Consumed));
        assert_eq!(&*r.echo, b"a");
        // Not flushed yet (ICANON — still in line buffer)
        assert!(drain_stdin().is_empty());
    }

    #[test_case]
    fn newline_flushes_line() {
        let mut tty = TtyState::new();
        tty.process_input_byte(b'h');
        tty.process_input_byte(b'i');
        let r = tty.process_input_byte(b'\n');
        assert_eq!(&*r.echo, b"\n");
        assert_eq!(drain_stdin(), b"hi\n");
    }

    #[test_case]
    fn cr_mapped_to_nl() {
        let mut tty = TtyState::new();
        tty.process_input_byte(b'x');
        let r = tty.process_input_byte(b'\r');
        assert_eq!(&*r.echo, b"\n");
        assert_eq!(drain_stdin(), b"x\n");
    }

    #[test_case]
    fn backspace_erases() {
        let mut tty = TtyState::new();
        tty.process_input_byte(b'a');
        tty.process_input_byte(b'b');
        let r = tty.process_input_byte(127);
        assert_eq!(&*r.echo, b"\x08 \x08");
        tty.process_input_byte(b'\n');
        assert_eq!(drain_stdin(), b"a\n");
    }

    #[test_case]
    fn backspace_on_empty_does_nothing() {
        let mut tty = TtyState::new();
        let r = tty.process_input_byte(127);
        assert!(r.echo.is_empty());
    }

    #[test_case]
    fn ctrl_c_generates_signal() {
        let mut tty = TtyState::new();
        tty.process_input_byte(b'x');
        let r = tty.process_input_byte(3); // Ctrl-C
        assert!(matches!(r.action, InputAction::Signal));
        // Line buffer should be cleared
        tty.process_input_byte(b'\n');
        assert_eq!(drain_stdin(), b"\n");
    }
}
