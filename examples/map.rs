use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;

use alloy::primitives::B256;
use alloy::providers::ProviderBuilder;
use alloy::providers::{Provider, WsConnect};
use alloy::rpc::types::eth::Transaction;
use alloy::rpc::types::Header;
use burberry::collector::BlockCollector;
use burberry::{
    collector::MempoolCollector, map_collector, map_executor, submit_action, ActionSubmitter,
    Engine, Executor, Strategy,
};

#[tokio::main]
async fn main() {
    let ws = WsConnect::new("wss://eth.merkle.io");
    let provider = ProviderBuilder::new()
        .connect_ws(ws)
        .await
        .expect("fail to create ws provider");

    let provider: Arc<dyn Provider<_>> = Arc::new(provider);

    let mut engine = Engine::new();

    let mempool_collector = MempoolCollector::new(Arc::clone(&provider));
    let block_collector = BlockCollector::new(Arc::clone(&provider));

    engine.add_collector(map_collector!(mempool_collector, Event::Transaction));
    engine.add_collector(map_collector!(block_collector, Event::Block));

    engine.add_strategy(Box::new(EchoStrategy));

    engine.add_executor(map_executor!(EchoExecutor::default(), Action::EchoBlock));
    engine.add_executor(map_executor!(
        EchoExecutor::default(),
        Action::EchoTransaction
    ));

    engine.run_and_join().await.unwrap()
}

pub struct EchoStrategy;

#[async_trait::async_trait]
impl Strategy<Event, Action> for EchoStrategy {
    async fn process_event(&mut self, event: Event, submitter: Arc<dyn ActionSubmitter<Action>>) {
        match event {
            Event::Block(block) => {
                submit_action!(submitter, Action::EchoBlock, block.number);
            }
            Event::Transaction(tx) => {
                submit_action!(submitter, Action::EchoTransaction, *tx.inner.tx_hash());
            }
        }
    }
}

#[derive(Default)]
pub struct EchoExecutor<T>(PhantomData<T>);

#[async_trait::async_trait]
impl<T: Debug + Send + Sync> Executor<T> for EchoExecutor<T> {
    async fn execute(&self, action: T) -> eyre::Result<()> {
        println!("action: {:?}", action);
        Ok(())
    }
}

#[derive(Debug, Clone)]
enum Event {
    Block(Header),
    Transaction(Transaction),
}

#[derive(Debug, Clone)]
enum Action {
    EchoBlock(u64),
    EchoTransaction(B256),
}
