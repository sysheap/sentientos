use serial_test::file_serial;
use tokio::io::AsyncWriteExt;

use crate::infra::qemu::{QemuInstance, QemuOptions};

#[file_serial]
#[tokio::test]
async fn udp() -> anyhow::Result<()> {
    let mut yaros = QemuInstance::start_with(QemuOptions::default().add_network_card(true)).await?;

    yaros
        .run_prog_waiting_for("udp", "Listening on 1234\n")
        .await
        .expect("udp program must succeed to start");

    let socket = tokio::net::UdpSocket::bind("localhost:0").await?;
    socket.connect("localhost:1234").await?;

    socket.send("42".as_bytes()).await?;
    yaros.stdout().assert_read_until("42").await;

    yaros
        .stdin()
        .write("Hello from YaROS!\n".as_bytes())
        .await?;

    let mut buf = [0; 128];
    let bytes = socket.recv(&mut buf).await?;
    let response = String::from_utf8_lossy(&buf[0..bytes]);

    assert_eq!(response, "Hello from YaROS!\n");

    socket.send("Finalize test".as_bytes()).await?;
    yaros.stdout().assert_read_until("Finalize test").await;

    Ok(())
}
