use std::sync::Arc;

use alloy::{
    primitives::{keccak256, Bytes},
    providers::Provider,
    transports::Transport,
};
use eyre::Result;
use async_trait::async_trait;

use crate::types::Executor;

pub struct RawTransactionSenderExecutor<T, P> {
    provider: Arc<P>,

    _phantom: std::marker::PhantomData<T>,
}

impl<T, P> RawTransactionSenderExecutor<T, P> {
    pub fn new(provider: Arc<P>) -> Self {
        Self {
            provider,

            _phantom: Default::default(),
        }
    }
}

#[async_trait]
impl<T: Transport + Clone, P: Provider<T>> Executor<Bytes> for RawTransactionSenderExecutor<T, P> {
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
