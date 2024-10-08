use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicU64, Arc};
use std::time::Duration;

use alloy::rpc::types::eth::BlockId;
use alloy::rpc::types::BlockTransactionsKind;
use alloy::transports::Transport;
use alloy::{providers::Provider, rpc::types::eth::Block};
use async_trait::async_trait;
use tracing::error;

use crate::types::{Collector, CollectorStream};

pub struct PollFullBlockCollector<T> {
    provider: Arc<dyn Provider<T>>,
    interval: Duration,
    current_block: AtomicU64,
}

impl<T> PollFullBlockCollector<T> {
    pub fn new(provider: Arc<dyn Provider<T>>, interval: Duration) -> Self {
        Self {
            provider,
            interval,
            current_block: AtomicU64::new(0),
        }
    }
}

#[async_trait]
impl<T: Clone + Transport> Collector<Block> for PollFullBlockCollector<T> {
    fn name(&self) -> &str {
        "PollFullBlockCollector"
    }

    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, Block>> {
        let stream = async_stream::stream! {
            loop {
                match self.provider.get_block(BlockId::latest(), BlockTransactionsKind::Full).await {
                    Ok(Some(block)) => {
                        let current_block = block.header.number;

                        let old_block = self
                            .current_block
                            .fetch_max(current_block, Ordering::Relaxed);

                        if old_block < current_block {
                            yield block;
                        }
                    }
                    Ok(None) => {
                        error!("latest block not found");
                    }
                    Err(e) => {
                        error!("fail to get latest block: {e:#}")
                    }
                };

                tokio::time::sleep(self.interval).await;
            }
        };

        Ok(Box::pin(stream))
    }
}
