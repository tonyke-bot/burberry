use std::sync::Arc;

use alloy::{providers::Provider, pubsub::PubSubFrontend, rpc::types::eth::Block};
use async_trait::async_trait;

use crate::types::{Collector, CollectorStream};

pub struct BlockCollector<T> {
    provider: Arc<dyn Provider<T>>,
}

impl<T> BlockCollector<T> {
    pub async fn new(provider: Arc<dyn Provider<T>>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl Collector<Block> for BlockCollector<PubSubFrontend> {
    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, Block>> {
        let stream = self.provider.subscribe_blocks().await?;

        Ok(Box::pin(stream.into_stream()))
    }
}
