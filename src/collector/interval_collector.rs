use std::time::{Duration, Instant};

use crate::types::{Collector, CollectorStream};
use async_trait::async_trait;

pub struct IntervalCollector {
    interval: Duration,
}

impl IntervalCollector {
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }
}

#[async_trait]
impl Collector<Instant> for IntervalCollector {
    fn name(&self) -> &str {
        "IntervalCollector"
    }

    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, Instant>> {
        let stream = async_stream::stream! {
            loop {
                tokio::time::sleep(self.interval).await;
                yield Instant::now();
            }
        };

        Ok(Box::pin(stream))
    }
}
