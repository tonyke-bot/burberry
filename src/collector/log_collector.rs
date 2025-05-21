use std::sync::Arc;

use alloy::{
    providers::Provider,
    rpc::types::eth::{Filter, Log},
};
use async_trait::async_trait;
use futures::StreamExt;

use crate::types::{Collector, CollectorStream};

pub struct LogCollector {
    provider: Arc<dyn Provider>,
    filter: Filter,
}

impl LogCollector {
    pub fn new(provider: Arc<dyn Provider>, filter: Filter) -> Self {
        Self { provider, filter }
    }
}

#[async_trait]
impl Collector<Log> for LogCollector {
    fn name(&self) -> &str {
        "LogCollector"
    }

    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, Log>> {
        let stream = self.provider.subscribe_logs(&self.filter).await?;
        let stream = stream.into_stream().filter_map(|v| async move { Some(v) });
        Ok(Box::pin(stream))
    }
}
