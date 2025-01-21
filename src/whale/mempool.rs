use log::{info, warn, error, debug};

use solana_client::nonblocking::rpc_client::RpcClient;

// Add futures
use futures::StreamExt;

// For error attributes, add at the top:
use thiserror::Error;
use solana_sdk::{
    signature::Signature,
    transaction::Transaction,
};
use std::{
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::mpsc,
    time::timeout,
};
use crate::SolanaConfig;

use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct TransactionLog {
    pub signature: Signature,
    pub raw_transaction: Transaction,
    pub timestamp: u64,
}

#[derive(Debug)]
pub enum MempoolError {
    #[error("RPC Connection Error: {0}")]
    RpcConnectionError(String),

    #[error("Websocket Connection Error: {0}")]
    WebsocketError(String),

    #[error("Timeout Error: {0}")]
    TimeoutError(String),
}

#[derive(Debug)]
pub struct MempoolMonitor {
    solana_config: SolanaConfig,
    rpc_urls: Vec<String>,
    websocket_urls: Vec<String>,
    transaction_sender: mpsc::Sender<TransactionLog>,
    transaction_receiver: mpsc::Receiver<TransactionLog>,
}

impl MempoolMonitor {
    pub fn new(solana_config: SolanaConfig) -> Self {
        let rpc_urls = vec![
            "https://api.mainnet-beta.solana.com".to_string(),
            "https://rpc.ankr.com/solana".to_string(),
        ];

        let websocket_urls = vec![
            solana_config.websocket_url(),
            "wss://rpc.ankr.com/solana_jsonrpc".to_string(),
        ];

        let (tx, rx) = mpsc::channel(100);

        Self {
            solana_config,
            rpc_urls,
            websocket_urls,
            transaction_sender: tx,
            transaction_receiver: rx,
        }
    }

    // RPC-based transaction monitoring
    async fn monitor_rpc_stream(
        solana_config: &SolanaConfig,
        rpc_url: String,
        sender: mpsc::Sender<TransactionLog>
    ) -> Result<(), MempoolError> {
        let rpc_client = solana_config.create_rpc_client();

        info!("Monitoring rpc stream");
        loop {
            // Fetch recent transactions
            match rpc_client.get_recent_transactions().await {
                Ok(transactions) => {
                    for tx in transactions {
                        info!("Transaction from RPC stream: {}", txn);
                        let log = TransactionLog {
                            signature: tx.signature,
                            raw_transaction: tx.transaction,
                            timestamp: chrono::Utc::now().timestamp() as u64,
                        };

                        // Send transaction log
                        let _ = sender.send(log).await;
                    }
                },
                Err(e) => {
                    eprintln!("RPC transaction fetch error: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }

            // Wait before next poll
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}