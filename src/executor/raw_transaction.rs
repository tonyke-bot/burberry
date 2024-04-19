use alloy::transports::Transport;
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

#[async_trait]
impl<T: Clone + Transport> Executor<Bytes> for RawTransactionSender<T> {
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
