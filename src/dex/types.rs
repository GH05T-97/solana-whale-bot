
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct DexTransaction {
    pub signature: String,
    pub program_id: Pubkey,
    pub instruction_data: Vec<u8>,
    pub accounts: Vec<Pubkey>,
    pub token_in: Option<Pubkey>,
    pub token_out: Option<Pubkey>,
    pub amount_in: Option<u64>,
    pub amount_out: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum DexProtocol {
    Jupiter,
    Raydium,
}

#[derive(Debug, Clone)]
pub enum TradeType {
    Buy {
        token: Pubkey,
        amount: u64,
        price: f64,
    },
    Sell {
        token: Pubkey,
        amount: u64,
        price: f64,
    },
    Unknown,
}

#[derive(Debug, Clone)]
pub struct DexTrade {
    pub protocol: DexProtocol,
    pub trade_type: TradeType,
    pub signature: String,
    pub timestamp: i64,
    pub slippage: f64,
    pub price_impact: f64,
}