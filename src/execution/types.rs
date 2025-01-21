use rust_decimal::Decimal;
// Common imports to add
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct OrderRequest {
    pub token: Pubkey,
    pub direction: OrderDirection,
    pub size: Decimal,
    pub price: Decimal,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub enum OrderDirection {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub enum OrderType {
    Market,
    Limit(Decimal),
}

#[derive(Debug, Clone)]
pub enum TimeInForce {
    GoodTilCancelled,
    ImmediateOrCancel,
    FillOrKill,
}

#[derive(Debug, Clone)]
pub enum OrderStatus {
    New,
    PartiallyFilled { filled_amount: Decimal },
    Filled { fill_price: Decimal },
    Cancelled,
    Failed { reason: String },
}

#[derive(Debug, Clone)]
pub struct OrderResult {
    pub order_id: String,
    pub status: OrderStatus,
    pub fills: Vec<Fill>,
    pub average_price: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Fill {
    pub price: Decimal,
    pub size: Decimal,
    pub timestamp: DateTime<Utc>,
    pub fee: Decimal,
}

pub struct SwapParams {
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub amount: u64,
}

#[derive(Debug, Clone)]
pub enum DexType {
    Jupiter,
    Raydium,
}