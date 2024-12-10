use anyhow::anyhow;
use std::process::Stdio;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use super::read_asserter::ReadAsserter;

pub struct QemuOptions {
    add_network_card: bool,
    assert_successful_startup: bool,
}

impl Default for QemuOptions {
    fn default() -> Self {
        Self {
            add_network_card: false,
            assert_successful_startup: true,
        }
    }
}

impl QemuOptions {
    pub fn add_network_card(mut self, value: bool) -> Self {
        self.add_network_card = value;
        self
    }
    pub fn assert_successful_startup(mut self, value: bool) -> Self {
        self.assert_successful_startup = value;
        self
    }

    fn apply(self, command: &mut Command) {
        if self.add_network_card {
            command.arg("--net");
        }
    }
}

pub struct QemuInstance {
    _instance: Child,
    stdin: ChildStdin,
    stdout: ReadAsserter<ChildStdout>,
}

impl QemuInstance {
    pub async fn start() -> anyhow::Result<Self> {
        Self::start_with(QemuOptions::default()).await
    }

    pub async fn start_with(options: QemuOptions) -> anyhow::Result<Self> {
        let mut command = Command::new("../qemu_wrapper.sh");

        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        let assert_successful_startup = options.assert_successful_startup;

        options.apply(&mut command);

        command.arg("target/riscv64gc-unknown-none-elf/release/kernel");

        let mut instance = command.spawn()?;

        let stdin = instance
            .stdin
            .take()
            .ok_or(anyhow!("Could not get stdin"))?;

        let stdout = instance
            .stdout
            .take()
            .ok_or(anyhow!("Could not get stdout"))?;

        let mut stdout = ReadAsserter::new(stdout);

        if assert_successful_startup {
            stdout.assert_read_until("Hello World from YaROS!").await;
            stdout.assert_read_until("kernel_init done!").await;
            stdout.assert_read_until("init process started").await;
            stdout
                .assert_read_until("### YaSH - Yet another Shell ###")
                .await;
            stdout.assert_read_until("$ ").await;
        }

        Ok(Self {
            _instance: instance,
            stdin,
            stdout,
        })
    }

    pub fn read_asserter(&mut self) -> &mut ReadAsserter<ChildStdout> {
        &mut self.stdout
    }

    pub fn stdin(&mut self) -> &mut ChildStdin {
        &mut self.stdin
    }
}
