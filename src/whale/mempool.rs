use log::{info, warn, error, debug};
use futures::StreamExt;
use thiserror::Error;
use solana_sdk::{
    signature::Signature,
    transaction::Transaction,
    signer::keypair::Keypair,
    hash::Hash,
};
use solana_client::rpc_client::RpcClient;
use solana_transaction_status::EncodedConfirmedBlock;
use std::time::Duration;

use tokio::{
    sync::mpsc,
    time::timeout,
};
use crate::SolanaConfig;
use solana_transaction_status::EncodedConfirmedBlock;
use solana_transaction_status::UiTransactionEncoding;

use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TransactionLog {
    pub signature: Signature,
    pub raw_transaction: Transaction,
    pub timestamp: u64,
}

#[derive(Debug, Error)]
pub enum MempoolError {
    #[error("RPC Connection Error: {0}")]
    RpcConnectionError(String),

    #[error("Websocket Connection Error: {0}")]
    WebsocketError(String),

    #[error("Timeout Error: {0}")]
    TimeoutError(String),
}

pub struct MempoolMonitor {
    solana_config: SolanaConfig,
    rpc_urls: Vec<String>,
    websocket_urls: Vec<String>,
    transaction_sender: mpsc::Sender<TransactionLog>,
    transaction_receiver: mpsc::Receiver<TransactionLog>,
}

// Remove Clone derive since Receiver can't be cloned
impl MempoolMonitor {
    pub fn new(solana_config: SolanaConfig) -> (Self, mpsc::Sender<TransactionLog>) {
        let rpc_urls = vec![
            "https://api.mainnet-beta.solana.com".to_string(),
            "https://rpc.ankr.com/solana".to_string(),
        ];

        let websocket_urls = vec![
            solana_config.websocket_url(),
            "wss://rpc.ankr.com/solana_jsonrpc".to_string(),
        ];

        let (tx, rx) = mpsc::channel(100);
        let tx_clone = tx.clone();

        (Self {
            solana_config,
            rpc_urls,
            websocket_urls,
            transaction_sender: tx,
            transaction_receiver: rx,
        }, tx_clone)
    }

    /// Start monitoring the mempool using RPC
    pub async fn monitor_mempool(&self) -> Result<(), MempoolError> {
        let rpc_client = Arc::new(self.solana_config.create_rpc_client());

        info!("Starting mempool monitoring");
        loop {
            // Fetch recent blockhash
            let blockhash = rpc_client.get_latest_blockhash()
                .map_err(|e| MempoolError::RpcConnectionError(e.to_string()))?;

            // Fetch transactions for the latest blockhash
            let block = rpc_client.get_block_with_encoding(
                blockhash,
                solana_transaction_status::UiTransactionEncoding::Base64
            )
            .map_err(|e| MempoolError::RpcConnectionError(e.to_string()))?;

            if let Some(block) = block {
                for tx in block.transactions {
                    if let Some(transaction) = tx.transaction {
                        let log = TransactionLog {
                            signature: transaction.signatures[0],
                            raw_transaction: transaction,
                            timestamp: chrono::Utc::now().timestamp() as u64,
                        };

                        // Send transaction log
                        if let Err(e) = self.transaction_sender.send(log).await {
                            error!("Failed to send transaction log: {:?}", e);
                        }
                    }
                }
            }

            // Wait before the next poll
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}