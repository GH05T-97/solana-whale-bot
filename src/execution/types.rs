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

#[derive(Clone, Debug, PartialEq)]
pub enum OrderDirection {
    Buy,
    Sell,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OrderType {
    Market,
    Limit(Decimal),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimeInForce {
    GoodTilCancelled,
    ImmediateOrCancel,
    FillOrKill,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OrderStatus {
    New,
    PartiallyFilled { filled_amount: Decimal },
    Filled { fill_price: Decimal },
    Cancelled,
    Failed { reason: String },
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