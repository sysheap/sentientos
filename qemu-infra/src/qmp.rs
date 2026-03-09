use std::path::Path;

use anyhow::anyhow;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

pub struct QmpClient {
    reader: BufReader<tokio::io::ReadHalf<UnixStream>>,
    writer: tokio::io::WriteHalf<UnixStream>,
}

impl QmpClient {
    pub async fn connect(socket_path: &Path) -> anyhow::Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        let (read_half, write_half) = tokio::io::split(stream);
        let mut client = Self {
            reader: BufReader::new(read_half),
            writer: write_half,
        };

        // Read greeting
        let greeting = client.read_line().await?;
        if !greeting.contains("\"QMP\"") {
            return Err(anyhow!("unexpected QMP greeting: {greeting}"));
        }

        // Negotiate capabilities
        client
            .send_command(r#"{"execute": "qmp_capabilities"}"#)
            .await?;

        Ok(client)
    }

    pub async fn screendump(&mut self, output_path: &Path) -> anyhow::Result<()> {
        let path_str = output_path.to_string_lossy();
        let cmd =
            format!(r#"{{"execute": "screendump", "arguments": {{"filename": "{path_str}"}}}}"#);
        self.send_command(&cmd).await
    }

    async fn send_command(&mut self, cmd: &str) -> anyhow::Result<()> {
        self.writer.write_all(cmd.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;

        let response = self.read_line().await?;
        if response.contains("\"error\"") {
            return Err(anyhow!("QMP error: {response}"));
        }
        Ok(())
    }

    async fn read_line(&mut self) -> anyhow::Result<String> {
        let mut line = String::new();
        loop {
            line.clear();
            let n = self.reader.read_line(&mut line).await?;
            if n == 0 {
                return Err(anyhow!("QMP connection closed"));
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Skip async events
            if trimmed.contains("\"event\"") {
                continue;
            }
            return Ok(trimmed.to_string());
        }
    }
}
