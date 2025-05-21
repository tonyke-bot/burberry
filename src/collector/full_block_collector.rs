use std::sync::Arc;
use std::time::Duration;

use alloy::{providers::Provider,  rpc::types::eth::Block};
use async_trait::async_trait;
use futures::StreamExt;
use tracing::{error, warn};

use crate::types::{Collector, CollectorStream};

pub struct FullBlockCollector {
    provider: Arc<dyn Provider>,
    retry_interval: Duration,
}

impl FullBlockCollector {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self::new_with_config(provider, Duration::from_millis(50))
    }

    /// Create a new `FullBlockCollector` with a custom retry interval. A retry will happen when the client returns
    /// "header not found"
    pub fn new_with_config(provider: Arc<dyn Provider>, retry_interval: Duration) -> Self {
        Self {
            provider,
            retry_interval,
        }
    }
}

#[async_trait]
impl Collector<Block> for FullBlockCollector {
    fn name(&self) -> &str {
        "FullBlockCollector"
    }

    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, Block>> {
        let mut stream = self.provider.subscribe_blocks().await?.into_stream();

        let mut attempts = 0;

        let stream = async_stream::stream! {
            while let Some(block) = stream.next().await {
                let block_number = block.number;

                loop {
                    match self.provider.get_block_by_number(block_number.into()).full().await {
                        Ok(Some(block)) => {
                            yield block;
                        }
                        Ok(None) => {
                            if attempts % 5 == 0 {
                                warn!(block = block_number, "block not found yet");
                            } else {
                                error!(block = block_number, "block not found yet");
                            }

                            attempts += 1;
                            tokio::time::sleep(self.retry_interval).await;
                            continue;
                        }
                        Err(e) => {
                            error!(block = block_number, "fail to get full block: {e:#}");
                        }
                    };

                    break;
                }
            }
        };

        Ok(Box::pin(stream))
    }
}
