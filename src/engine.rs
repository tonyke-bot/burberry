use std::{fmt::Debug, sync::Arc};

use crate::{
    action_submitter::ActionChannelSubmitter,
    types::{Collector, Executor, Strategy},
};
use eyre::Context;
use futures::StreamExt;
use tokio::{
    sync::broadcast::{self, error::RecvError, Sender},
    task::JoinSet,
};
use tracing::{debug, error, warn};

pub struct Engine<E, A> {
    collectors: Vec<Box<dyn Collector<E>>>,
    strategies: Vec<Box<dyn Strategy<E, A>>>,
    executors: Vec<Box<dyn Executor<A>>>,

    event_channel_capacity: usize,
    action_channel_capacity: usize,
}

impl<E, A> Engine<E, A> {
    pub fn new() -> Self {
        Self {
            collectors: vec![],
            strategies: vec![],
            executors: vec![],
            event_channel_capacity: 512,
            action_channel_capacity: 512,
        }
    }

    pub fn with_event_channel_capacity(mut self, capacity: usize) -> Self {
        self.event_channel_capacity = capacity;
        self
    }

    pub fn with_action_channel_capacity(mut self, capacity: usize) -> Self {
        self.action_channel_capacity = capacity;
        self
    }

    pub fn strategy_count(&self) -> usize {
        self.strategies.len()
    }

    pub fn executor_count(&self) -> usize {
        self.executors.len()
    }
}

impl<E, A> Default for Engine<E, A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E, A> Engine<E, A>
where
    E: Send + Sync + Clone + 'static,
    A: Send + Sync + Clone + Debug + 'static,
{
    pub fn add_collector(&mut self, collector: Box<dyn Collector<E>>) {
        self.collectors.push(collector);
    }

    pub fn add_strategy(&mut self, strategy: Box<dyn Strategy<E, A>>) {
        self.strategies.push(strategy);
    }

    pub fn add_executor(&mut self, executor: Box<dyn Executor<A>>) {
        self.executors.push(executor);
    }

    pub async fn run_and_join(self) -> Result<(), Box<dyn std::error::Error>> {
        let mut js = self.run().await?;

        while let Some(event) = js.join_next().await {
            if let Err(err) = event {
                error!("task terminated unexpectedly: {err:#}");
            }
        }

        Ok(())
    }

    pub async fn run(self) -> Result<JoinSet<()>, Box<dyn std::error::Error>> {
        let (event_sender, _): (Sender<E>, _) = broadcast::channel(self.event_channel_capacity);
        let (action_sender, _): (Sender<A>, _) = broadcast::channel(self.action_channel_capacity);

        let mut set = JoinSet::new();

        if self.executors.is_empty() {
            return Err("no executors".into());
        }

        if self.collectors.is_empty() {
            return Err("no collectors".into());
        }

        if self.strategies.is_empty() {
            return Err("no strategies".into());
        }

        // Spawn executors in separate threads.
        for executor in self.executors {
            let mut receiver = action_sender.subscribe();

            set.spawn(async move {
                debug!(name = executor.name(), "starting executor... ");

                loop {
                    match receiver.recv().await {
                        Ok(action) => match executor.execute(action).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!(name = executor.name(), "error executing action: {}", e)
                            }
                        },
                        Err(RecvError::Closed) => {
                            error!(name = executor.name(), "action channel closed!");
                            break;
                        }
                        Err(RecvError::Lagged(num)) => {
                            warn!(name = executor.name(), "action channel lagged by {num}")
                        }
                    }
                }
            });
        }

        // Spawn strategies in separate threads.
        for mut strategy in self.strategies {
            let mut event_receiver = event_sender.subscribe();
            let action_sender = action_sender.clone();

            let action_submitter = Arc::new(ActionChannelSubmitter::new(action_sender));

            strategy
                .sync_state(action_submitter.clone())
                .await
                .wrap_err("fail to sync state")?;

            set.spawn(async move {
                debug!(name = strategy.name(), "starting strategy... ");

                loop {
                    match event_receiver.recv().await {
                        Ok(event) => {
                            strategy
                                .process_event(event, action_submitter.clone())
                                .await
                        }
                        Err(RecvError::Closed) => {
                            error!(name = strategy.name(), "event channel closed!");
                            break;
                        }
                        Err(RecvError::Lagged(num)) => {
                            warn!(name = strategy.name(), "event channel lagged by {num}")
                        }
                    }
                }
            });
        }

        // Spawn collectors in separate threads.
        for collector in self.collectors {
            let event_sender = event_sender.clone();

            set.spawn(async move {
                debug!(name = collector.name(), "starting collector... ");
                let mut event_stream = collector.get_event_stream().await.unwrap();

                while let Some(event) = event_stream.next().await {
                    if let Err(e) = event_sender.send(event) {
                        error!(name = collector.name(), "error sending event: {e:#}");
                    }
                }

                error!(name = collector.name(), "event stream ended!");
            });
        }

        Ok(set)
    }
}
