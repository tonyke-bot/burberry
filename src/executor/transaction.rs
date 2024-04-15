use std::collections::HashMap;

use alloy::{
    network::{eip2718::Encodable2718, EthereumSigner, TransactionBuilder},
    primitives::{keccak256, Address, Bytes},
    providers::{Provider, RootProvider},
    rpc::types::eth::TransactionRequest,
    signers::wallet::LocalWallet,
};

use crate::types::Executor;

pub struct TransactionSender {
    signers: HashMap<Address, EthereumSigner>,
    provider: Box<dyn Provider>,
    tx_submission_provider: Option<Box<dyn Provider>>,
}

impl TransactionSender {
    pub fn new(provider: Box<dyn Provider>, signers: Vec<LocalWallet>) -> Self {
        let signers: HashMap<_, _> = signers
            .into_iter()
            .map(|s| (s.address(), EthereumSigner::new(s)))
            .collect();

        for signer in signers.keys() {
            tracing::info!("setting up signer {:#x}", signer);
        }

        Self {
            provider,
            signers,
            tx_submission_provider: None,
        }
    }
}

impl TransactionSender {
    pub fn new_with_dedicated_tx_submission_endpoint(
        provider: Box<dyn Provider>,
        tx_submission_provider: Box<dyn Provider>,
        signers: Vec<LocalWallet>,
    ) -> Self {
        let signers: HashMap<_, _> = signers
            .into_iter()
            .map(|s| (s.address(), EthereumSigner::new(s)))
            .collect();

        for signer in signers.keys() {
            tracing::info!("setting up signer {:#x}", signer);
        }

        Self {
            provider,
            signers,
            tx_submission_provider: Some(tx_submission_provider),
        }
    }

    pub fn new_http_dedicated(
        provider: Box<dyn Provider>,
        tx_submission_endpoint: &str,
        signers: Vec<LocalWallet>,
    ) -> Self {
        let tx_submission_provider =
            Box::new(RootProvider::<_>::new_http(tx_submission_endpoint.parse().unwrap()).boxed());
        Self::new_with_dedicated_tx_submission_endpoint(provider, tx_submission_provider, signers)
    }

    pub fn new_with_flashbots(provider: Box<dyn Provider>, signers: Vec<LocalWallet>) -> Self {
        Self::new_http_dedicated(provider, "https://rpc.flashbots.net/fast", signers)
    }

    pub fn new_with_bsc_bloxroute(provider: Box<dyn Provider>, signers: Vec<LocalWallet>) -> Self {
        Self::new_http_dedicated(provider, "https://bsc.rpc.blxrbdn.com", signers)
    }

    pub fn new_with_48club(provider: Box<dyn Provider>, signers: Vec<LocalWallet>) -> Self {
        Self::new_http_dedicated(provider, "https://rpc-bsc.48.club", signers)
    }

    pub fn new_with_polygon_bloxroute(
        provider: Box<dyn Provider>,
        signers: Vec<LocalWallet>,
    ) -> Self {
        Self::new_http_dedicated(provider, "https://polygon.rpc.blxrbdn.com", signers)
    }

    pub fn new_with_arbitrum_sequencer(
        provider: Box<dyn Provider>,
        signers: Vec<LocalWallet>,
    ) -> Self {
        Self::new_http_dedicated(provider, "https://arb1-sequencer.arbitrum.io/rpc", signers)
    }
}

#[async_trait::async_trait]
impl Executor<TransactionRequest> for TransactionSender {
    async fn execute(&self, action: TransactionRequest) -> eyre::Result<()> {
        let account = match action.from {
            Some(v) => v,
            None => {
                tracing::error!("missing sender address");
                return Ok(());
            }
        };

        let signer = match self.signers.get(&account) {
            Some(v) => v,
            None => {
                tracing::error!("missing signer for {:#x}", account);
                return Ok(());
            }
        };

        let nonce = match self.provider.get_transaction_count(account, None).await {
            Ok(v) => v,
            Err(err) => {
                tracing::error!(?account, "failed to get nonce: {err:#}");
                return Ok(());
            }
        };

        let raw_tx: Bytes = match action.nonce(nonce).build(signer).await {
            Ok(v) => v.encoded_2718().into(),
            Err(err) => {
                tracing::error!(?account, nonce, "failed to build tx: {err:#}");
                return Ok(());
            }
        };

        tracing::debug!(?account, nonce, tx = ?raw_tx, "signed tx");

        let send_result = match &self.tx_submission_provider {
            Some(dedicated_provider) => dedicated_provider
                .send_raw_transaction(&raw_tx)
                .await
                .map(|v| *v.tx_hash()),
            None => self
                .provider
                .send_raw_transaction(&raw_tx)
                .await
                .map(|v| *v.tx_hash()),
        };

        let tx_hash = match send_result {
            Ok(v) => v,
            Err(err) => {
                let hash = keccak256(&raw_tx);
                tracing::error!(?account, nonce, tx = ?hash, "failed to send tx: {err:#}");
                return Ok(());
            }
        };

        tracing::info!(?account, nonce, "sent tx: {:#x}", tx_hash);

        Ok(())
    }
}
