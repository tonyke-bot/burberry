use std::sync::Arc;

use alloy::{providers::Provider, pubsub::PubSubFrontend, rpc::types::eth::Block};
use async_trait::async_trait;

use crate::types::{Collector, CollectorStream};

pub struct BlockCollector<PubSubFrontend> {
    provider: Arc<dyn Provider<PubSubFrontend>>,
}

impl<PubSubFrontend> BlockCollector<PubSubFrontend> {
    pub fn new(provider: Arc<dyn Provider<PubSubFrontend>>) -> Self {
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
