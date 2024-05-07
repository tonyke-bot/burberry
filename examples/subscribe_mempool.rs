use std::sync::Arc;
use std::time::Duration;

use alloy::{providers::ProviderBuilder, rpc::client::WsConnect};
use burberry::{collector::MempoolCollector, Collector};
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let ws = WsConnect::new("wss://eth.merkle.io");
    let provider = ProviderBuilder::new()
        .on_ws(ws)
        .await
        .expect("fail to create ws provider");

    let collector = MempoolCollector::new(Arc::new(provider));
    let mut stream = collector
        .get_event_stream()
        .await
        .expect("fail to get event stream");

    while let Some(tx) = stream.next().await {
        tokio::time::sleep(Duration::from_secs(500)).await;
        println!("received tx: {:?}", tx);
    }
}
