use std::sync::Arc;

use alloy::{
    primitives::B256, providers::Provider, pubsub::PubSubFrontend, rpc::types::eth::Transaction,
};
use async_channel::bounded;
use async_trait::async_trait;
use tokio::sync::mpsc::channel;
use tracing::{error, trace};

use crate::types::{Collector, CollectorStream};

pub struct MempoolCollector {
    provider: Arc<dyn Provider<PubSubFrontend>>,
    concurrency: usize,
}

impl MempoolCollector {
    pub fn new(provider: Arc<dyn Provider<PubSubFrontend>>) -> Self {
        Self::new_with_concurrency(provider, 10)
    }

    pub fn new_with_concurrency(
        provider: Arc<dyn Provider<PubSubFrontend>>,
        concurrency: usize,
    ) -> Self {
        Self {
            provider,
            concurrency,
        }
    }
}

#[async_trait]
impl Collector<Transaction> for MempoolCollector {
    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, Transaction>> {
        let (ch_hash_tx, ch_hash_rx) = bounded::<B256>(2048);
        let (ch_tx_tx, mut ch_tx_rx) = channel::<Transaction>(128);

        for _ in 0..self.concurrency {
            let provider = self.provider.clone();
            let ch_hash_rx = ch_hash_rx.clone();
            let ch_tx_tx = ch_tx_tx.clone();

            tokio::spawn(async move {
                while let Ok(hash) = ch_hash_rx.recv().await {
                    trace!(target: "pending-tx-fetcher", %hash, "fetching pending transaction");

                    let tx = match provider.get_transaction_by_hash(hash).await {
                        Ok(tx) => tx,
                        Err(err) => {
                            error!(target: "pending-tx-fetcher", %hash, "fail to get pending transaction by hash: {err:#}");
                            continue;
                        }
                    };

                    trace!(target: "pending-tx-fetcher", %hash, "transaction: {tx:?}");
                    ch_tx_tx.send(tx).await.unwrap();
                }
            });
        }

        {
            let provider = self.provider.clone();

            tokio::spawn(async move {
                let mut stream = provider
                    .subscribe_pending_transactions()
                    .await
                    .expect("fail to subscribe pending transactions");

                while let Ok(tx) = stream.recv().await {
                    ch_hash_tx.send(tx).await.unwrap();
                }
            });
        }

        let stream = async_stream::stream! {
            while let Some(tx) = ch_tx_rx.recv().await {
                yield tx;
            }
        };

        Ok(Box::pin(stream))
    }
}
