use anyhow::anyhow;
use std::{
    io::{self, ErrorKind},
    os::unix::process::CommandExt,
    process::Stdio,
};
use tokio::{
    io::{AsyncWrite, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};

pub struct QemuInstance {
    instance: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl QemuInstance {
    pub fn start() -> anyhow::Result<Self> {
        Self::start_with(&[])
    }
    pub fn start_with(args: &[&str]) -> anyhow::Result<Self> {
        let args_with_kernel = args.iter().chain(std::iter::once(
            &"target/riscv64gc-unknown-none-elf/release/kernel",
        ));

        let mut instance = Command::new("../qemu_wrapper.sh")
            .args(args_with_kernel)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = instance
            .stdin
            .take()
            .ok_or(anyhow!("Could not get stdin"))?;

        let stdout = instance
            .stdout
            .take()
            .ok_or(anyhow!("Could not get stdout"))?;

        let stdout = BufReader::new(stdout);

        Ok(Self {
            instance,
            stdin,
            stdout,
        })
    }

    pub fn stdout(&mut self) -> &mut BufReader<ChildStdout> {
        &mut self.stdout
    }
}
