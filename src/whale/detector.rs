use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::env;

use tokio::sync::RwLock;
use super::{
    config::WhaleConfig,
    types::{Transaction, WhaleMovement, MovementType},
    cache::WhaleCache,
    mempool::MempoolMonitor,

};

use crate::dex::types::{DexTransaction, TradeType, DexProtocol, DexTrade};
use crate::dex::DexAnalyzer;

use crate::execution::TradeExecutor;
use crate::solana_config::SolanaConfig;
use crate::strategy::types::{
    StrategyConfig,
    RiskParams,
    TradeSignal
};
use crate::strategy::StrategyAnalyzer;
use rust_decimal::Decimal;


use rust_decimal::Decimal;
use crate::SolanaConfig;

#[derive(Clone)]
struct PendingTransaction {
    signature: String,
    from_address: String,
    to_address: String,
    amount: u64,
    timestamp: u64,
}

pub struct WhaleDetector {
    config: WhaleConfig,
    recent_movements: Arc<RwLock<VecDeque<WhaleMovement>>>,
    known_whales: Arc<RwLock<HashSet<String>>>,
    cache: WhaleCache,
    mempool: MempoolMonitor,
    dex_analyzer: DexAnalyzer,
    strategy_analyzer: StrategyAnalyzer,
    trade_executor: TradeExecutor,
}

impl WhaleDetector {
    pub fn new(config: WhaleConfig) -> Self {
        let known_whales = Arc::new(RwLock::new(config.tracked_addresses.clone()));
        let recent_movements = Arc::new(RwLock::new(VecDeque::with_capacity(1000)));

        // Initialize strategy config with our 1 SOL parameters
        let strategy_config = StrategyConfig {
            risk_params: RiskParams::default(),
            min_whale_success_rate: 0.6,
            min_liquidity: Decimal::new(1000000000, 9), // 1 SOL minimum liquidity
            max_slippage: Decimal::new(2, 2),           // 2% max slippage
            max_price_impact: Decimal::new(1, 2),       // 1% max price impact
            total_portfolio_sol: Decimal::new(1000000000, 9), // 1 SOL portfolio
        };

        let wallet_env = env::var("WALLET_KEYPAIR_PATH").expect("WALLET_KEYPAIR_PATH must be set");
        let solana_config = SolanaConfig::mainnet_default(wallet_env);

        Self {
            config,
            recent_movements,
            known_whales,
            cache: WhaleCache::new(),
            mempool: MempoolMonitor::new(solana_config), /// change the paramters
            dex_analyzer: DexAnalyzer::new(),
            strategy_analyzer: StrategyAnalyzer::new(strategy_config),
            trade_executor
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Start mempool monitoring
        let mempool_clone = self.mempool.clone();
        tokio::spawn(async move {
            mempool_clone.monitor_mempool().await;
        });

        // Start transaction processing
        self.process_transactions().await;

        Ok(())
    }

    pub async fn process_transactions(&self) {

        while let Ok(transaction) = rx.recv().await {
            // Process confirmed transactions in parallel
            let detector = self.clone();
            tokio::spawn(async move {
                if let Some(movement) = detector.analyze_transaction(transaction).await {
                    detector.handle_whale_movement(movement).await;
                }
            });
        }
    }

    async fn analyze_transaction(&self, transaction: Transaction) -> Option<WhaleMovement> {
        // Check if it's a whale transaction
        if !self.is_whale_transaction(&transaction).await {
            return None;
        }

        // Convert and analyze DEX transaction
        let dex_transaction = DexTransaction::from(transaction.clone());
        let dex_trade = self.dex_analyzer.analyze_transaction(dex_transaction).await?;

        // Get whale address and calculate confidence
        let (whale_address, confidence) = tokio::join!(
            self.determine_whale_address(&transaction),
            self.calculate_confidence(&transaction)
        );

        // Convert DEX trade to movement type
        let movement_type = match dex_trade.trade_type {
            TradeType::Buy { token, amount, price } => MovementType::TokenSwap {
                action: "buy".to_string(),
                token_address: token,
                amount,
                price,
            },
            TradeType::Sell { token, amount, price } => MovementType::TokenSwap {
                action: "sell".to_string(),
                token_address: token,
                amount,
                price,
            },
            TradeType::Unknown => return None, // Skip non-DEX trades
        };

        // Cache the results
        self.cache.update_whale_data(
            &whale_address,
            movement_type.clone(),
            confidence
        ).await;

        Some(WhaleMovement {
            transaction,
            whale_address,
            movement_type,
            confidence,
        })
    }

    // async fn handle_pending_transaction(&self, pending_tx: PendingTransaction) {
    //     if self.is_potential_whale_transaction(&pending_tx).await {
    //         // Pre-warm cache
    //         self.cache.preload_data(&pending_tx).await;

    //         // Notify subscribers of potential whale movement
    //         self.notify_pending_movement(&pending_tx).await;
    //     }
    // }

    async fn handle_whale_movement(&self, movement: WhaleMovement) {
        // Update recent movements
        let mut movements = self.recent_movements.write().await;
        movements.push_front(movement.clone());

        // Keep only recent movements
        while movements.len() > 1000 {
            movements.pop_back();
        }

        // Update cache with confirmed movement
        self.cache.update_movement_history(movement.clone()).await;

        // Generate trade signal using strategy analyzer
        if let Some(trade_signal) = self.strategy_analyzer.analyze_whale_movement(&movement).await {
            // Log the trade signal
            log::info!(
                "Whale Movement Trade Signal: Token: {:?}, Direction: {:?}, Size: {:?}",
                trade_signal.token,
                trade_signal.direction,
                trade_signal.size
            );

            // Execute trade using injected TradeExecutor
            let trade_executor = self.trade_executor.clone();
            let cloned_signal = trade_signal.clone();

            tokio::spawn(async move {
                match trade_executor.execute_trade(cloned_signal).await {
                    Ok(order_result) => {
                        log::info!("Trade executed successfully: {:?}", order_result);
                    },
                    Err(execution_error) => {
                        log::error!("Trade execution failed: {:?}", execution_error);
                    }
                }
            });
    }

    pub async fn is_whale_transaction(&self, transaction: &Transaction) -> bool {
        // Check cache first
        if let Some(is_whale) = self.cache.get_whale_status(&transaction.from_address).await {
            return is_whale;
        }

        let known_whales = self.known_whales.read().await;
        let is_whale = known_whales.contains(&transaction.from_address) ||
                      known_whales.contains(&transaction.to_address);

        // Update cache
        self.cache.set_whale_status(&transaction.from_address, is_whale).await;

        return is_whale
    }

    async fn determine_whale_address(&self, transaction: &Transaction) -> String {
        let known_whales = self.known_whales.read().await;
        if known_whales.contains(&transaction.from_address) {
            transaction.from_address.clone()
        } else {
            transaction.to_address.clone()
        }
    }

    async fn determine_movement_type(&self, transaction: &Transaction) -> MovementType {
        // Check cache for similar transactions
        if let Some(cached_type) = self.cache.get_movement_type(&transaction.signature).await {
            return cached_type;
        }

        MovementType::Unknown
    }

    async fn calculate_confidence(&self, transaction: &Transaction) -> f64 {
        // Check cache for similar transactions
        if let Some(confidence) = self.cache.get_confidence(&transaction.signature).await {
            return confidence;
        }

        let confidence = if transaction.amount > self.config.minimum_transaction * 10 {
            0.9
        } else {
            0.7
        };

        // Cache the result
        self.cache.set_confidence(&transaction.signature, confidence).await;

        confidence
    }

    // async fn is_potential_whale_transaction(&self, pending_tx: &PendingTransaction) -> bool {
    //     // Quick check using cached data
    //     if let Some(is_whale) = self.cache.get_whale_status(&pending_tx.from_address).await {
    //         return is_whale && pending_tx.amount >= self.config.minimum_transaction;
    //     }

    //     let known_whales = self.known_whales.read().await;
    //     known_whales.contains(&pending_tx.from_address) &&
    //     pending_tx.amount >= self.config.minimum_transaction
    // }

    // async fn notify_pending_movement(&self, pending_tx: &PendingTransaction) {
    //     // Implementation for notifying about potential whale movements
    //     // This could be used for even faster reaction times
    // }

    // pub async fn add_whale(&self, address: String) {
    //     let mut known_whales = self.known_whales.write().await;
    //     known_whales.insert(address.clone());
    //     self.cache.set_whale_status(&address, true).await;
    // }

    // pub async fn remove_whale(&self, address: &str) {
    //     let mut known_whales = self.known_whales.write().await;
    //     known_whales.remove(address);
    //     self.cache.set_whale_status(address, false).await;
    // }

    // pub async fn get_recent_movements(&self) -> Vec<WhaleMovement> {
    //     self.recent_movements.read().await.iter().cloned().collect()
    // }
}}