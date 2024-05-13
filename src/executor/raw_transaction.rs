use alloy::providers::ProviderBuilder;
use alloy::transports::{BoxTransport, Transport};
use alloy::{
    primitives::{keccak256, Bytes},
    providers::Provider,
};
use async_trait::async_trait;
use eyre::Result;
use std::sync::Arc;

use crate::types::Executor;

pub struct RawTransactionSender<T> {
    provider: Arc<dyn Provider<T>>,
}

impl<T> RawTransactionSender<T> {
    pub fn new(provider: Arc<dyn Provider<T>>) -> Self {
        Self { provider }
    }
}

impl RawTransactionSender<BoxTransport> {
    pub fn new_http(url: &str) -> Self {
        let provider = ProviderBuilder::new().on_http(url.parse().unwrap());
        let provider = Arc::new(provider.boxed());
        Self { provider }
    }

    pub fn new_with_flashbots() -> Self {
        Self::new_http("https://rpc.flashbots.net/fast")
    }

    pub fn new_with_bsc_bloxroute() -> Self {
        Self::new_http("https://bsc.rpc.blxrbdn.com")
    }

    pub fn new_with_48club() -> Self {
        Self::new_http("https://rpc-bsc.48.club")
    }

    pub fn new_with_polygon_bloxroute() -> Self {
        Self::new_http("https://polygon.rpc.blxrbdn.com")
    }

    pub fn new_with_arbitrum_sequencer() -> Self {
        Self::new_http("https://arb1-sequencer.arbitrum.io/rpc")
    }
}

#[async_trait]
impl<T: Clone + Transport> Executor<Bytes> for RawTransactionSender<T> {
    fn name(&self) -> &str {
        "RawTransactionSender"
    }

    async fn execute(&self, action: Bytes) -> Result<()> {
        let send_result = self.provider.send_raw_transaction(&action).await;

        match send_result {
            Ok(tx) => {
                tracing::info!(tx = ?tx.tx_hash(), "sent tx");
            }
            Err(err) => {
                let tx_hash = keccak256(&action);
                tracing::error!(tx = ?tx_hash, "failed to send tx: {:#}", err);
            }
        }

        Ok(())
    }
}
