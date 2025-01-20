#[derive(Clone, Debug)]
pub struct Transaction {
    pub signature: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: u64,
    pub timestamp: u64,
    pub block_number: u64,
}

#[derive(Clone, Debug)]
pub struct WhaleMovement {
    pub transaction: Transaction,
    pub whale_address: String,
    pub movement_type: MovementType,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
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
    Unknown,
}

#[derive(Debug, Clone)]
pub enum MovementType {
    TokenSwap {
        action: String,
        token_address: Pubkey,
        amount: f64,
        price: f64,
    },
    // Add other movement types if needed
}