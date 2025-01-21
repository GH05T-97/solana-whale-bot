// Common imports to add
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};


#[derive(Clone, Debug, Default)]
pub struct Transaction {
    pub signature: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    pub timestamp: u64,
    pub block_number: u64,
}

#[derive(Clone, Debug, Default)]
pub struct WhaleMovement {
    pub transaction: Transaction,
    pub whale_address: String,
    pub movement_type: MovementType,
    pub confidence: f64,
    pub price: f64,
}

#[derive(Clone, Debug, Default)]
pub enum TradeType {
    Buy {
        token: Pubkey,
        amount: f64,
        price: f64
    },
    Sell {
        token: Pubkey,
        amount: f64,
        price: f64
    },
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default)]
pub enum MovementType {
    TokenSwap {
        action: String,
        token_address: Pubkey,
        amount: f64,
        price: f64,
    },
    #[default]
    Unknown,
    // Add other movement types if needed
}