use tokio::io::AsyncWriteExt;

use crate::infra::qemu::{QemuInstance, QemuOptions};

#[tokio::test]
async fn should_exit_program() -> anyhow::Result<()> {
    let mut solaya =
        QemuInstance::start_with(QemuOptions::default().add_network_card(true)).await?;

    solaya
        .run_prog_waiting_for("udp", "Listening on 1234")
        .await?;

    solaya.ctrl_c_and_assert_prompt().await?;

    let output = solaya.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}

#[tokio::test]
async fn should_rerun_udp_after_ctrl_c() -> anyhow::Result<()> {
    let mut solaya =
        QemuInstance::start_with(QemuOptions::default().add_network_card(true)).await?;

    solaya
        .run_prog_waiting_for("udp", "Listening on 1234")
        .await?;
    solaya.ctrl_c_and_assert_prompt().await?;

    solaya
        .run_prog_waiting_for("udp", "Listening on 1234")
        .await?;
    solaya.ctrl_c_and_assert_prompt().await?;

    let output = solaya.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}

#[tokio::test]
async fn should_not_exit_sosh() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    solaya.stdin().write_all(&[0x03]).await?;
    solaya.stdin().flush().await?;

    let output = solaya.run_prog("prog1").await?;
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}

#[tokio::test]
async fn self_kill_invokes_handler() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("sigtest self-kill").await?;
    assert!(output.contains("caught signal 2"), "output: {output}");
    assert!(output.contains("OK"), "output: {output}");

    Ok(())
}

#[tokio::test]
async fn ctrl_c_invokes_handler() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    solaya
        .run_prog_waiting_for("sigtest wait-for-signal", "waiting")
        .await?;

    // Send Ctrl+C — this should deliver SIGINT to the handler, not kill the process.
    // The handler sets a flag, the program prints "caught signal 2\nOK\n" and exits.
    solaya.stdin().write_all(&[0x03]).await?;
    solaya.stdin().flush().await?;

    // The program should print its output and then exit, returning to shell prompt.
    let output = solaya.stdout().assert_read_until("$ ").await?;
    let output = String::from_utf8_lossy(&output);
    assert!(output.contains("caught signal 2"), "output: {output}");

    Ok(())
}

#[tokio::test]
async fn sig_ign_survives_ctrl_c() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    solaya
        .run_prog_waiting_for("sigtest ignore", "waiting")
        .await?;

    // Send Ctrl+C — with SIG_IGN, this should NOT kill the process.
    solaya.stdin().write_all(&[0x03]).await?;
    solaya.stdin().flush().await?;

    // The process is sleeping for 2 seconds. It should print "OK" and exit normally.
    let output = solaya.stdout().assert_read_until("$ ").await?;
    let output = String::from_utf8_lossy(&output);
    assert!(output.contains("OK"), "output: {output}");

    Ok(())
}
