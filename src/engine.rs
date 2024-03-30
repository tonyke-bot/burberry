use std::{fmt::Debug, sync::Arc};

use tokio::{
    sync::broadcast::{self, error::RecvError, Sender},
    task::JoinSet,
};
use tokio_stream::StreamExt;

use crate::{
    action_submitter::ActionChannelSubmitter,
    types::{Collector, Executor, Strategy},
};

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

    pub async fn run(self) -> Result<JoinSet<()>, Box<dyn std::error::Error>> {
        let (event_sender, _): (Sender<E>, _) = broadcast::channel(self.event_channel_capacity);
        let (action_sender, _): (Sender<A>, _) = broadcast::channel(self.action_channel_capacity);

        let mut set = JoinSet::new();

        tracing::info!("burberry engine started");

        // Spawn executors in separate threads.
        for executor in self.executors {
            let mut receiver = action_sender.subscribe();
            set.spawn(async move {
                tracing::info!("starting executor... ");
                loop {
                    match receiver.recv().await {
                        Ok(action) => match executor.execute(action).await {
                            Ok(_) => {}
                            Err(e) => tracing::error!("error executing action: {}", e),
                        },
                        Err(e) => tracing::error!("error receiving action: {}", e),
                    }
                }
            });
        }

        // Spawn strategies in separate threads.
        for mut strategy in self.strategies {
            let mut event_receiver = event_sender.subscribe();
            let action_sender = action_sender.clone();

            let action_submitter = Arc::new(ActionChannelSubmitter::new(action_sender));

            set.spawn(async move {
                tracing::info!("starting strategy... ");

                loop {
                    match event_receiver.recv().await {
                        Ok(event) => strategy.process_event(event, action_submitter.clone()).await,
                        Err(RecvError::Closed) => panic!("event channel closed"),
                        Err(RecvError::Lagged(num)) => tracing::warn!("event channel lagged by {num}"),
                    }
                }
            });
        }

        // Spawn collectors in separate threads.
        for collector in self.collectors {
            let event_sender = event_sender.clone();
            set.spawn(async move {
                tracing::info!("starting collector... ");
                let mut event_stream = collector.get_event_stream().await.unwrap();
                while let Some(event) = event_stream.next().await {
                    match event_sender.send(event) {
                        Ok(_) => {}
                        Err(e) => tracing::error!("error sending event: {}", e),
                    }
                }
            });
        }

        Ok(set)
    }
}
