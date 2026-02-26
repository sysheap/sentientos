use tokio::io::AsyncWriteExt;

use crate::infra::qemu::{QemuInstance, QemuOptions};

#[tokio::test]
async fn udp() -> anyhow::Result<()> {
    let mut solaya =
        QemuInstance::start_with(QemuOptions::default().add_network_card(true)).await?;

    solaya
        .run_prog_waiting_for("udp", "Listening on 1234\n")
        .await
        .expect("udp program must succeed to start");

    let port = solaya.network_port().expect("Network must be enabled");
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await?;
    socket.connect(format!("127.0.0.1:{}", port)).await?;

    socket.send("42\n".as_bytes()).await?;
    solaya.stdout().assert_read_until("42\n").await?;

    solaya
        .stdin()
        .write_all("Hello from Solaya!\n".as_bytes())
        .await?;
    solaya.stdin().flush().await?;

    let mut buf = [0; 128];
    let bytes = socket.recv(&mut buf).await?;
    let response = String::from_utf8_lossy(&buf[0..bytes]);

    assert_eq!(response, "Hello from Solaya!\n");

    socket.send("Finalize test\n".as_bytes()).await?;
    solaya.stdout().assert_read_until("Finalize test\n").await?;

    Ok(())
}
