use solana_client::{
    nonblocking::{
        websocket::{WebSocket, WebSocketClient},
        rpc_client::RpcClient
    },
    rpc_config::{RpcTransactionLogsFilter, RpcTransactionLogsConfig},
};
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
use thiserror::Error;
use futures::StreamExt;

#[derive(Debug, Clone)]
pub struct TransactionLog {
    pub signature: Signature,
    pub raw_transaction: Transaction,
    pub timestamp: u64,
}

#[derive(Error, Debug)]
pub enum MempoolError {
    #[error("RPC Connection Error: {0}")]
    RpcConnectionError(String),

    #[error("Websocket Connection Error: {0}")]
    WebsocketError(String),

    #[error("Timeout Error: {0}")]
    TimeoutError(String),
}

pub struct MempoolMonitor {
    rpc_urls: Vec<String>,
    websocket_urls: Vec<String>,
    transaction_sender: mpsc::Sender<TransactionLog>,
    transaction_receiver: mpsc::Receiver<TransactionLog>,
}

impl MempoolMonitor {
    pub fn new() -> Self {
        let rpc_urls = vec![
            "https://api.mainnet-beta.solana.com".to_string(),
            "https://rpc.ankr.com/solana".to_string(),
        ];

        let websocket_urls = vec![
            "wss://api.mainnet-beta.solana.com".to_string(),
            "wss://rpc.ankr.com/solana_jsonrpc".to_string(),
        ];

        let (tx, rx) = mpsc::channel(100);

        Self {
            rpc_urls,
            websocket_urls,
            transaction_sender: tx,
            transaction_receiver: rx,
        }
    }

    // Main monitoring method
    pub async fn start_monitoring(&self) -> Result<mpsc::Receiver<TransactionLog>, MempoolError> {
        let (tx, mut rx) = mpsc::channel(100);

        // Spawn monitoring tasks for each RPC and Websocket URL
        for rpc_url in &self.rpc_urls {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::monitor_rpc_stream(rpc_url.clone(), tx_clone).await {
                    eprintln!("RPC monitoring error: {:?}", e);
                }
            });
        }

        for ws_url in &self.websocket_urls {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::monitor_websocket_stream(ws_url.clone(), tx_clone).await {
                    eprintln!("Websocket monitoring error: {:?}", e);
                }
            });
        }

        Ok(rx)
    }

    // RPC-based transaction monitoring
    async fn monitor_rpc_stream(
        rpc_url: String,
        sender: mpsc::Sender<TransactionLog>
    ) -> Result<(), MempoolError> {
        let rpc_client = RpcClient::new(rpc_url);

        loop {
            // Fetch recent transactions
            match rpc_client.get_recent_transactions().await {
                Ok(transactions) => {
                    for tx in transactions {
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

    // Websocket-based real-time monitoring
    async fn monitor_websocket_stream(
        ws_url: String,
        sender: mpsc::Sender<TransactionLog>
    ) -> Result<(), MempoolError> {
        let ws_client = WebSocketClient::new(&ws_url)
            .await
            .map_err(|e| MempoolError::WebsocketError(e.to_string()))?;

        let mut logs_stream = ws_client.logs_subscribe(
            RpcTransactionLogsFilter::All,
            RpcTransactionLogsConfig::default()
        ).await.map_err(|e| MempoolError::WebsocketError(e.to_string()))?;

        while let Some(log_result) = logs_stream.next().await {
            match log_result {
                Ok(log_entry) => {
                    // Convert log entry to TransactionLog
                    let log = TransactionLog {
                        signature: log_entry.signature,
                        raw_transaction: log_entry.transaction, // Adjust based on actual log entry structure
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    };

                    // Send transaction log
                    let _ = sender.send(log).await;
                },
                Err(e) => {
                    eprintln!("Websocket error: {:?}", e);
                    break;
                }
            }
        }

        Err(MempoolError::WebsocketError("Connection lost".to_string()))
    }
}