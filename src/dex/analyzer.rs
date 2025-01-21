use super::types::{DexTransaction, TradeType, DexTrade, DexProtocol};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;


use crate::dex::protocols::{
    JUPITER_PROGRAM_ID,
    RAYDIUM_PROGRAM_ID,
};

#[derive(Clone, Debug, Default)]
pub struct DexAnalyzer {
    supported_dexes: HashSet<Pubkey>,
}

const MINIMUM_TRADE_AMOUNT: u64 = 1_000_000;

impl DexAnalyzer {
    pub fn new() -> Self {
        let mut supported_dexes = HashSet::new();

        // Add supported DEX program IDs
        supported_dexes.insert(Pubkey::from_str(JUPITER_PROGRAM_ID).unwrap());
        supported_dexes.insert(Pubkey::from_str(RAYDIUM_PROGRAM_ID).unwrap());

        Self {
            supported_dexes,
        }
    }

    pub async fn analyze_transaction(&self, transaction: DexTransaction) -> Option<DexTrade> {
        // Check if this is a DEX transaction
        if !self.supported_dexes.contains(&transaction.program_id) {
            return None;
        }

        // Analyze based on protocol
        let protocol = self.identify_protocol(&transaction.program_id);
        let trade = match protocol {
            DexProtocol::Jupiter => jupiter::analyze_trade(&transaction).await,
            DexProtocol::Raydium => raydium::analyze_trade(&transaction).await,
            DexProtocol::Unknown => return None,
        }?;

        // Filter for buy/sell of tracked tokens only
        if !self.is_relevant_trade(&trade) {
            return None;
        }

        Some(trade)
    }

    fn is_relevant_trade(&self, trade: &DexTrade) -> bool {
        // Implement your criteria for relevant trades
        match trade.trade_type {
            TradeType::Buy { amount, .. } => {
                // Example criteria: Only consider trades above a certain size
                amount > MINIMUM_TRADE_AMOUNT
            },
            TradeType::Sell { amount, .. } => {
                // Example criteria: Only consider trades above a certain size
                amount > MINIMUM_TRADE_AMOUNT
            },
            TradeType::Unknown => false,
        }
    }


    fn identify_protocol(&self, program_id: &Pubkey) -> DexProtocol {
        match program_id.to_string().as_str() {
            jupiter_program_id => DexProtocol::Jupiter,
            raydium_program_id => DexProtocol::Raydium,
            _ => DexProtocol::Unknown,
        }
    }
}