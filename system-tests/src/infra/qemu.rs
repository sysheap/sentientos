use anyhow::anyhow;
use std::net::UdpSocket;
use std::process::{ExitStatus, Stdio};
use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin, ChildStdout, Command},
};

use super::{PROMPT, read_asserter::ReadAsserter};

fn find_available_port() -> anyhow::Result<u16> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    Ok(socket.local_addr()?.port())
}

pub struct QemuOptions {
    add_network_card: bool,
    use_smp: bool,
}

impl Default for QemuOptions {
    fn default() -> Self {
        Self {
            add_network_card: false,
            use_smp: true,
        }
    }
}

impl QemuOptions {
    pub fn add_network_card(mut self, value: bool) -> Self {
        self.add_network_card = value;
        self
    }
    pub fn use_smp(mut self, value: bool) -> Self {
        self.use_smp = value;
        self
    }

    fn apply(self, command: &mut Command) -> Option<u16> {
        let mut network_port = None;
        if self.add_network_card {
            let port = find_available_port().expect("Failed to allocate network port");
            command.args(["--net", &port.to_string()]);
            network_port = Some(port);
        }
        if self.use_smp {
            command.arg("--smp");
        }
        network_port
    }
}

pub struct QemuInstance {
    instance: Child,
    stdin: ChildStdin,
    stdout: ReadAsserter<ChildStdout>,
    network_port: Option<u16>,
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

        let network_port = options.apply(&mut command);

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

        stdout
            .assert_read_until("Hello World from SentientOS!")
            .await;
        stdout.assert_read_until("kernel_init done!").await;
        stdout.assert_read_until("init process started").await;
        stdout
            .assert_read_until("### SeSH - Sentient Shell ###")
            .await;
        stdout.assert_read_until(PROMPT).await;

        Ok(Self {
            instance,
            stdin,
            stdout,
            network_port,
        })
    }

    pub fn stdout(&mut self) -> &mut ReadAsserter<ChildStdout> {
        &mut self.stdout
    }

    pub fn stdin(&mut self) -> &mut ChildStdin {
        &mut self.stdin
    }

    pub fn network_port(&self) -> Option<u16> {
        self.network_port
    }

    pub async fn ctrl_c_and_assert_prompt(&mut self) -> anyhow::Result<String> {
        self.stdin().write_all(&[0x03]).await?;
        self.stdout().assert_read_until(PROMPT).await;
        Ok(String::new())
    }

    pub async fn wait_for_qemu_to_exit(mut self) -> anyhow::Result<ExitStatus> {
        // Ensure stdin is closed so the child isn't stuck waiting on
        // input while the parent is waiting for it to exit.
        drop(self.stdin);
        drop(self.stdout);

        Ok(self.instance.wait().await?)
    }

    pub async fn run_prog(&mut self, prog_name: &str) -> anyhow::Result<String> {
        self.run_prog_waiting_for(prog_name, PROMPT).await
    }

    pub async fn run_prog_waiting_for(
        &mut self,
        prog_name: &str,
        wait_for: &str,
    ) -> anyhow::Result<String> {
        let command = format!("{}\n", prog_name);

        self.stdin.write_all(command.as_bytes()).await?;

        let result = self.stdout.assert_read_until(wait_for).await;
        let trimmed_result = &result[command.len()..result.len() - wait_for.len()];

        Ok(String::from_utf8_lossy(trimmed_result).into_owned())
    }

    pub async fn write_and_wait_for(&mut self, text: &str, wait: &str) -> anyhow::Result<()> {
        self.stdin().write_all(text.as_bytes()).await?;
        self.stdout().assert_read_until(wait).await;
        Ok(())
    }
}
