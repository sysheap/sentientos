use crate::infra::qemu::QemuInstance;

#[tokio::test]
async fn panic() -> anyhow::Result<()> {
    let mut sentientos = QemuInstance::start().await?;
    let output = sentientos
        .run_prog_waiting_for("panic", "Time to attach gdb ;) use 'just attach'")
        .await?;

    assert!(output.contains("Hello from Panic! Triggering kernel panic"));
    assert!(output.contains("Kernel Page Tables Pagetables at"));
    assert!(output.contains("<rust_begin_unwind+"));
    assert!(output.contains("<handle_exception+"));
    assert!(output.contains("<asm_handle_exception+"));
    assert!(output
        .contains("[info][kernel::debugging] Current Thread: tid=3 pid=3 process_name=panic pc=0x2160 state=Running in_kernel_mode=false waiting_on_syscall=false"));

    Ok(())
}
