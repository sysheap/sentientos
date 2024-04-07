use std::time::Duration;

#[tokio::test]
async fn boot() {
    tokio::time::sleep(Duration::from_millis(100)).await;
}
