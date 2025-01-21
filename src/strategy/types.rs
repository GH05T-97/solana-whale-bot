use rust_decimal::Decimal;
// Common imports to add
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Default)]
pub enum TradeDirection {
    #[default]
    Long,
    Short,
}

#[derive(Clone, Debug, Default)]
pub struct TradeSignal {
    pub direction: TradeDirection,
    pub token: Pubkey,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
    pub confidence: f64,
    pub source_whale: String,
    pub price_impact: Decimal,
    pub estimated_slippage: Decimal,
}

#[derive(Clone, Default)]
pub struct StrategyConfig {
    pub risk_params: RiskParams,
    pub min_whale_success_rate: f64,
    pub min_liquidity: Decimal,
    pub max_slippage: Decimal,
    pub max_price_impact: Decimal,
    pub total_portfolio_sol: Decimal, // 1 SOL
}

#[derive(Debug, Clone)]
pub struct RiskParams {
    pub max_position_size: Decimal,    // Maximum 0.2 SOL per trade (20% of portfolio)
    pub max_loss_per_trade: Decimal,   // Maximum 0.02 SOL loss per trade (2% of portfolio)
    pub max_total_risk: Decimal,       // Maximum 0.5 SOL at risk (50% of portfolio)
    pub min_confidence: f64,           // Minimum confidence to take a trade
    pub max_concurrent_trades: u8,     // Maximum 2 trades at once
}

impl Default for RiskParams {
    fn default() -> Self {
        Self {
            max_position_size: Decimal::new(200000000, 9),  // 0.2 SOL
            max_loss_per_trade: Decimal::new(20000000, 9),  // 0.02 SOL
            max_total_risk: Decimal::new(500000000, 9),     // 0.5 SOL
            min_confidence: 0.8,                            // 80% confidence minimum
            max_concurrent_trades: 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TradeInfo {
    pub signal: TradeSignal,
    pub risk_amount: Decimal,
    pub entry_time: DateTime<Utc>,
    pub status: TradeStatus,
}

#[derive(Debug, Clone)]
pub enum TradeStatus {
    Active,
    Closed { exit_price: Decimal, pnl: Decimal },
    StopLossHit,
    TakeProfitHit,
}

#[derive(Debug, Default)]
pub struct PortfolioState {
    pub total_value: Decimal,
    pub available_balance: Decimal,
    pub unrealized_pnl: Decimal,
}