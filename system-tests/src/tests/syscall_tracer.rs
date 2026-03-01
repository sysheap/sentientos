use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn traced_process_emits_enter_exit() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("prog2").await?;

    // The write syscall trace should bracket the actual program output:
    //   [SYSCALL ENTER] tid=N write(fd: 1, buf: 0x..., count: 0x11)
    //   Hello from Prog2
    //   [SYSCALL EXIT]  tid=N write = 17
    let lines: Vec<&str> = output.lines().collect();
    let write_enter_idx = lines
        .iter()
        .position(|l| l.contains("write(fd: 1, buf: 0x") && l.contains(", count: 0x11)"))
        .expect("missing write ENTER line");
    assert_eq!(lines[write_enter_idx + 1], "Hello from Prog2");
    assert!(
        lines[write_enter_idx + 2].contains("write = 17"),
        "missing write EXIT line"
    );

    assert!(output.contains("exit_group(status: 0)"));

    Ok(())
}

#[tokio::test]
async fn untraced_process_has_no_trace_output() -> anyhow::Result<()> {
    let mut solaya = QemuInstance::start().await?;

    let output = solaya.run_prog("prog1").await?;

    assert!(!output.contains("[SYSCALL"));
    assert_eq!(output, "Hello from Prog1\n");

    Ok(())
}
