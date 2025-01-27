use rust_decimal::Decimal;
// Common imports to add
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Default)]
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

#[derive(Clone, Debug)]
pub enum OrderDirection {
    Buy,
    Sell,
}

impl Default for OrderDirection {
    fn default() -> Self {
        OrderDirection::Buy
    }
}

#[derive(Clone, Debug)]
pub enum OrderType {
    Market,
    Limit,
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::Market
    }
}

#[derive(Clone, Debug)]
pub enum TimeInForce {
    GoodTilCancelled,
    ImmediateOrCancel,
    FillOrKill,
}

impl Default for TimeInForce {
    fn default() -> Self {
        TimeInForce::GoodTilCancelled
    }
}


#[derive(Clone, Debug)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

impl Default for OrderStatus {
    fn default() -> Self {
        OrderStatus::New
    }
}

#[derive(Clone, Debug, Default)]
pub struct OrderResult {
    pub order_id: String,
    pub status: OrderStatus,
    pub fills: Vec<Fill>,
    pub average_price: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Default)]
pub struct Fill {
    pub price: Decimal,
    pub size: Decimal,
    pub timestamp: DateTime<Utc>,
    pub fee: Decimal,
}

#[derive(Clone, Debug, Default)]
pub struct SwapParams {
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub amount: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DexType {
    Jupiter,
    Raydium,
}