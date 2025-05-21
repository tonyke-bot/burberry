use std::sync::Arc;

use crate::types::{Collector, CollectorStream};
use alloy::transports::{RpcError, TransportErrorKind};
use alloy::{primitives::B256, providers::Provider, rpc::types::eth::Transaction};
use async_trait::async_trait;
use eyre::WrapErr;
use futures::prelude::{stream::FuturesUnordered, Stream};
use futures::{FutureExt, StreamExt};
use std::future::Future;
use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
};
use tracing::error;

pub struct MempoolCollector {
    provider: Arc<dyn Provider>,
}

impl MempoolCollector {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl Collector<Transaction> for MempoolCollector {
    fn name(&self) -> &str {
        "MempoolCollector"
    }

    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, Transaction>> {
        let stream = self
            .provider
            .subscribe_pending_transactions()
            .await
            .wrap_err("fail to subscribe to pending transaction stream")?
            .into_stream();

        let stream = TransactionStream::new(self.provider.as_ref(), stream, 256);
        let stream = stream.filter_map(|res| async move { res.ok() });

        Ok(Box::pin(stream))
    }
}

/// Errors `TransactionStream` can throw
#[derive(Debug, thiserror::Error)]
pub enum GetTransactionError {
    #[error("Failed to get transaction `{0}`: {1}")]
    ProviderError(B256, RpcError<TransportErrorKind>),

    /// `get_transaction` resulted in a `None`
    #[error("Transaction `{0}` not found")]
    NotFound(B256),
}

pub(crate) type TransactionFut<'a> = Pin<Box<dyn Future<Output = TransactionResult> + Send + 'a>>;

pub(crate) type TransactionResult = Result<Transaction, GetTransactionError>;

/// Drains a stream of transaction hashes and yields entire `Transaction`.
#[must_use = "streams do nothing unless polled"]
pub struct TransactionStream<'a, St> {
    /// Currently running futures pending completion.
    pub(crate) pending: FuturesUnordered<TransactionFut<'a>>,
    /// Temporary buffered transaction that get started as soon as another future finishes.
    pub(crate) buffered: VecDeque<B256>,
    /// The provider that gets the transaction
    pub(crate) provider: &'a dyn Provider,
    /// A stream of transaction hashes.
    pub(crate) stream: St,
    /// Marks if the stream is done
    stream_done: bool,
    /// max allowed futures to execute at once.
    pub(crate) max_concurrent: usize,
}

impl<'a, St> TransactionStream<'a, St> {
    /// Create a new `TransactionStream` instance
    pub fn new(provider: &'a dyn Provider, stream: St, max_concurrent: usize) -> Self {
        Self {
            pending: Default::default(),
            buffered: Default::default(),
            provider,
            stream,
            stream_done: false,
            max_concurrent,
        }
    }

    /// Push a future into the set
    pub(crate) fn push_tx(&mut self, tx: B256) {
        let fut = self
            .provider
            .root()
            .raw_request::<_, Option<Transaction>>("eth_getTransactionByHash".into(), (tx,))
            .then(move |res| match res {
                Ok(Some(tx)) => futures::future::ok(tx),
                Ok(None) => futures::future::err(GetTransactionError::NotFound(tx)),
                Err(err) => futures::future::err(GetTransactionError::ProviderError(tx, err)),
            });
        self.pending.push(Box::pin(fut));
    }
}

impl<'a, St> Stream for TransactionStream<'a, St>
where
    St: Stream<Item = B256> + Unpin + 'a,
{
    type Item = TransactionResult;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        // drain buffered transactions first
        while this.pending.len() < this.max_concurrent {
            if let Some(tx) = this.buffered.pop_front() {
                this.push_tx(tx);
            } else {
                break;
            }
        }

        if !this.stream_done {
            loop {
                match Stream::poll_next(Pin::new(&mut this.stream), cx) {
                    Poll::Ready(Some(tx)) => {
                        if this.pending.len() < this.max_concurrent {
                            this.push_tx(tx);
                        } else {
                            this.buffered.push_back(tx);
                        }
                    }
                    Poll::Ready(None) => {
                        this.stream_done = true;
                        break;
                    }
                    _ => break,
                }
            }
        }

        // poll running futures
        if let tx @ Poll::Ready(Some(_)) = this.pending.poll_next_unpin(cx) {
            return tx;
        }

        if this.stream_done && this.pending.is_empty() {
            // all done
            return Poll::Ready(None);
        }

        Poll::Pending
    }
}
