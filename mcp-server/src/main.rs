use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};

mod server;

use server::QemuMcpServer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mcp_server=info".parse()?),
        )
        .init();

    let server = QemuMcpServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
