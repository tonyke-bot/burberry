use std::sync::Arc;

use alloy::{
    providers::Provider,
    pubsub::PubSubFrontend,
    rpc::types::eth::{Filter, Log},
};
use async_trait::async_trait;
use futures::StreamExt;

use crate::types::{Collector, CollectorStream};

pub struct LogCollector<T, P> {
    provider: Arc<P>,
    filter: Filter,

    _phantom: std::marker::PhantomData<T>,
}

impl<T, P> LogCollector<T, P> {
    pub fn new(provider: Arc<P>, filter: Filter) -> Self {
        Self {
            provider,
            filter,
            _phantom: Default::default(),
        }
    }
}

#[async_trait]
impl<P: Provider<PubSubFrontend>> Collector<Log> for LogCollector<PubSubFrontend, P> {
    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, Log>> {
        let stream = self.provider.subscribe_logs(&self.filter).await?;
        let stream = stream.into_stream().filter_map(|v| async move { Some(v) });
        Ok(Box::pin(stream))
    }
}
