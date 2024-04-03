use std::sync::Arc;

use alloy::{
    providers::Provider,
    pubsub::PubSubFrontend,
    rpc::types::eth::{Filter, Log},
};
use async_trait::async_trait;
use futures::StreamExt;

use crate::types::{Collector, CollectorStream};

pub struct LogCollector<T> {
    provider: Arc<dyn Provider<T>>,
    filter: Filter,
}

impl<T> LogCollector<T> {
    pub fn new(provider: Arc<dyn Provider<T>>, filter: Filter) -> Self {
        Self { provider, filter }
    }
}

#[async_trait]
impl Collector<Log> for LogCollector<PubSubFrontend> {
    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, Log>> {
        let stream = self.provider.subscribe_logs(&self.filter).await?;
        let stream = stream.into_stream().filter_map(|v| async move { Some(v) });
        Ok(Box::pin(stream))
    }
}
