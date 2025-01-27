
use solana_sdk::pubkey::Pubkey;

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
    pub amount: f64,
    pub token_address: Pubkey,
    pub slippage: f64,
    pub price_impact: f64,
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