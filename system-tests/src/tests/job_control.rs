use qemu_infra::PROMPT;
use tokio::io::AsyncWriteExt;

use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn ctrl_d_exits_cat() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    // Start cat
    solaya.stdin().write_all(b"cat\n").await?;
    solaya.stdin().flush().await?;
    solaya.stdout().assert_read_until("cat\n").await?;

    // Send Ctrl+D on empty line to signal EOF
    solaya.stdin().write_all(&[0x04]).await?;
    solaya.stdin().flush().await?;

    // cat should exit and shell prompt should return
    solaya.stdout().assert_read_until(PROMPT).await?;

    // Verify shell still works
    let output = solaya.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}
