use std::time::Duration;

use crate::infra::qemu::{QemuInstance, QemuOptions};
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn doom_starts() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start_with(QemuOptions::default().framebuffer(true)).await?;

    solaya.stdin().write_all(b"doom\n").await?;
    solaya.stdin().flush().await?;

    // Wait for doom to extract WAD and start rendering
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Read whatever output is available
    let output = solaya.stdout().read_available().await;
    let output_str = String::from_utf8_lossy(&output);
    eprintln!("=== DOOM OUTPUT ===\n{}\n=== END ===", output_str);

    // Doom is now running (blocking the shell). Send Ctrl+C to kill it.
    solaya.ctrl_c_and_assert_prompt().await?;

    Ok(())
}
