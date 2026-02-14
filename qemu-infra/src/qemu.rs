use std::{
    net::UdpSocket,
    path::PathBuf,
    process::{ExitStatus, Stdio},
    time::Duration,
};

use anyhow::anyhow;
use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin, ChildStdout, Command},
};

use crate::{PROMPT, read_asserter::ReadAsserter};

fn find_available_port() -> anyhow::Result<u16> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    Ok(socket.local_addr()?.port())
}

pub fn project_root() -> anyhow::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("qemu_wrapper.sh").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err(anyhow!(
                "Could not find project root (no qemu_wrapper.sh in any parent directory)"
            ));
        }
    }
}

pub struct QemuOptions {
    add_network_card: bool,
    use_smp: bool,
    enable_gdb: bool,
}

impl Default for QemuOptions {
    fn default() -> Self {
        let enable_gdb = std::env::var("SENTIENTOS_ENABLE_GDB").is_ok();
        Self {
            add_network_card: false,
            use_smp: true,
            enable_gdb,
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
    pub fn enable_gdb(mut self, value: bool) -> Self {
        self.enable_gdb = value;
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
        if self.enable_gdb {
            command.arg("--gdb");
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
        let root = project_root()?;
        let wrapper = root.join("qemu_wrapper.sh");
        let mut command = Command::new(&wrapper);

        command
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        let gdb_enabled = options.enable_gdb;
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
        if gdb_enabled {
            stdout = stdout.with_timeout(Duration::from_secs(3600));
        }

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
        self.stdin().flush().await?;
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
        self.stdin.flush().await?;

        let result = self.stdout.assert_read_until(wait_for).await;
        let trimmed_result = &result[command.len()..result.len() - wait_for.len()];

        Ok(String::from_utf8_lossy(trimmed_result).into_owned())
    }

    pub async fn write_and_wait_for(&mut self, text: &str, wait: &str) -> anyhow::Result<()> {
        self.stdin().write_all(text.as_bytes()).await?;
        self.stdin().flush().await?;
        self.stdout().assert_read_until(wait).await;
        Ok(())
    }
}
