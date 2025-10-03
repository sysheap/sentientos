use serial_test::file_serial;
use tokio::io::AsyncWriteExt;

use crate::infra::qemu::{QemuInstance, QemuOptions};

#[file_serial]
#[tokio::test]
async fn udp() -> anyhow::Result<()> {
    let mut sentientos =
        QemuInstance::start_with(QemuOptions::default().add_network_card(true)).await?;

    sentientos
        .run_prog_waiting_for("udp", "Listening on 1234\n")
        .await
        .expect("udp program must succeed to start");

    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await?;
    socket.connect("127.0.0.1:1234").await?;

    socket.send("42\n".as_bytes()).await?;
    sentientos.stdout().assert_read_until("42\n").await;

    sentientos
        .stdin()
        .write_all("Hello from SentientOS!\n".as_bytes())
        .await?;

    let mut buf = [0; 128];
    let bytes = socket.recv(&mut buf).await?;
    let response = String::from_utf8_lossy(&buf[0..bytes]);

    assert_eq!(response, "Hello from SentientOS!\n");

    socket.send("Finalize test\n".as_bytes()).await?;
    sentientos
        .stdout()
        .assert_read_until("Finalize test\n")
        .await;

    Ok(())
}
