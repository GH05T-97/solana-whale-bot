// src/whale/monitor.rs
use tokio::sync::broadcast;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;

pub struct HybridMonitor {
    rpc_client: RpcClient,
    webhook_receiver: WebhookReceiver,
    transaction_sender: broadcast::Sender<Transaction>,
    known_signatures: HashSet<String>,  // To prevent duplicate processing
}

impl HybridMonitor {
    pub async fn start(&self) -> Result<(), Error> {
        // Create channels for both sources
        let (tx, _) = broadcast::channel(10000); // Buffer size for high throughput

        // Spawn WebSocket monitor
        let ws_tx = tx.clone();
        tokio::spawn(async move {
            self.monitor_websocket(ws_tx).await;
        });

        // Spawn webhook monitor
        let webhook_tx = tx.clone();
        tokio::spawn(async move {
            self.monitor_webhooks(webhook_tx).await;
        });

        // Spawn RPC monitor as fallback
        let rpc_tx = tx.clone();
        tokio::spawn(async move {
            self.monitor_rpc(rpc_tx).await;
        });

        Ok(())
    }

    async fn monitor_websocket(&self, tx: broadcast::Sender<Transaction>) {
        let ws = connect_to_solana_websocket().await;

        while let Some(msg) = ws.next().await {
            if let Ok(transaction) = self.parse_ws_message(msg) {
                // Process immediately due to low latency
                if !self.known_signatures.contains(&transaction.signature) {
                    self.known_signatures.insert(transaction.signature.clone());
                    tx.send(transaction).unwrap();
                }
            }
        }
    }

    async fn monitor_webhooks(&self, tx: broadcast::Sender<Transaction>) {
        while let Some(webhook_event) = self.webhook_receiver.next().await {
            if let Ok(transaction) = self.parse_webhook_event(webhook_event) {
                if !self.known_signatures.contains(&transaction.signature) {
                    self.known_signatures.insert(transaction.signature.clone());
                    tx.send(transaction).unwrap();
                }
            }
        }
    }

    async fn monitor_rpc(&self, tx: broadcast::Sender<Transaction>) {
        loop {
            // Poll less frequently as this is our fallback
            let signatures = self.rpc_client
                .get_signatures_for_address(&self.config.program_id)
                .await?;

            for sig in signatures {
                if !self.known_signatures.contains(&sig.signature.to_string()) {
                    if let Ok(transaction) = self.get_transaction_details(sig).await {
                        self.known_signatures.insert(transaction.signature.clone());
                        tx.send(transaction).unwrap();
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}