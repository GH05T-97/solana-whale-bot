use crate::dex::types::{DexTransaction, TradeType, DexTrade, DexProtocol};
// Common imports to add
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Default)]
pub struct RaydiumSwapParams {
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub pool_id: Pubkey,
}

pub async fn analyze_trade(transaction: &DexTransaction) -> Option<DexTrade> {
    let swap_params = parse_raydium_swap(transaction)?;

    let (token_in, token_out) = get_raydium_tokens(transaction)?;
    let trade_type = determine_trade_type(
        token_in,
        token_out,
        swap_params.amount_in,
        swap_params.min_amount_out
    );

    Some(DexTrade {
        protocol: DexProtocol::Raydium,
        trade_type,
        signature: transaction.signature.clone(),
        timestamp: transaction.block_time?,
        slippage: calculate_raydium_slippage(swap_params),
        price_impact: calculate_price_impact_raydium(swap_params),
    })
}

fn parse_raydium_swap(transaction: &DexTransaction) -> Option<RaydiumSwapParams> {
    let data = transaction.instruction_data.as_slice();

    // Raydium swap instruction layout
    match data.get(0)? {
        // Swap instruction discriminator
        9 => {
            Some(RaydiumSwapParams {
                amount_in: u64::from_le_bytes(data[1..9].try_into().ok()?),
                min_amount_out: u64::from_le_bytes(data[9..17].try_into().ok()?),
                pool_id: Pubkey::new(&data[17..49]),
            })
        },
        _ => None,
    }
}