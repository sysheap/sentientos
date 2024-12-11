use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn panic() -> anyhow::Result<()> {
    let mut yaros = QemuInstance::start().await?;
    let output = yaros
        .run_prog_waiting_for("panic", "Time to attach gdb ;) use 'just attach'")
        .await?;

    assert!(output.contains("Hello from Panic! Triggering kernel panic"));
    assert!(output.contains("Kernel Page Tables Pagetables at"));
    assert!(output.contains("<rust_begin_unwind+"));
    assert!(output.contains("<kernel::syscalls::handle_syscall+"));
    assert!(output
        .contains("[info][kernel::debugging] Current Process: PID=2 NAME=panic STATE=Runnabl"));

    Ok(())
}
