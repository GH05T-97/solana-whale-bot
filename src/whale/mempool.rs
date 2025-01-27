use log::{info, warn, error, debug};
use futures::StreamExt;
use thiserror::Error;  // Add this for proper error handling

use solana_sdk::{
    signature::Signature,
    transaction::Transaction,
    signer::keypair::Keypair,  // Add this for Keypair
};
use std::time::Duration;

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

#[derive(Debug, Error)]  
pub enum MempoolError {
    #[error("RPC Connection Error: {0}")]
    RpcConnectionError(String),

    #[error("Websocket Connection Error: {0}")]
    WebsocketError(String),

    #[error("Timeout Error: {0}")]
    TimeoutError(String),
}

#[derive(Clone)]
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

    /// Start monitoring the mempool using RPC
    pub async fn monitor_mempool(&self) -> Result<(), MempoolError> {
        let rpc_client = self.solana_config.create_rpc_client();

        info!("Starting mempool monitoring");
        loop {
            // Fetch recent blockhashes and transactions
            match rpc_client.get_recent_blockhash().await {
                Ok((blockhash, _)) => {
                    // Fetch transactions for the latest blockhash
                    match rpc_client.get_block(blockhash).await {
                        Ok(block) => {
                            for tx in block.transactions {
                                let log = TransactionLog {
                                    signature: tx.transaction.signatures[0],
                                    raw_transaction: tx.transaction,
                                    timestamp: chrono::Utc::now().timestamp() as u64,
                                };

                                // Send transaction log
                                if let Err(e) = self.transaction_sender.send(log).await {
                                    error!("Failed to send transaction log: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to fetch block: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to fetch recent blockhash: {:?}", e);
                }
            }

            // Wait before the next poll
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}