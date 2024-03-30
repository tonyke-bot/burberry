use std::sync::Arc;

use alloy::{providers::Provider, pubsub::PubSubFrontend, rpc::types::eth::Block};
use async_trait::async_trait;

use crate::types::{Collector, CollectorStream};

pub struct BlockCollector<T, P> {
    provider: Arc<P>,

    _phantom: std::marker::PhantomData<T>,
}

impl<T, P> BlockCollector<T, P> {
    pub async fn new(provider: Arc<P>) -> Self {
        Self {
            provider,
            _phantom: Default::default(),
        }
    }
}

#[async_trait]
impl<P: Provider<PubSubFrontend>> Collector<Block> for BlockCollector<PubSubFrontend, P> {
    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, Block>> {
        let stream = self.provider.subscribe_blocks().await?;

        Ok(Box::pin(stream.into_stream()))
    }
}
