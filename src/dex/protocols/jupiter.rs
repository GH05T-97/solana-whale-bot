use crate::dex::types::{DexTransaction, TradeType, DexTrade, DexProtocol};
use solana_sdk::instruction::Instruction;
// Common imports to add
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Default)]
pub struct JupiterSwapParams {
    pub in_amount: u64,
    pub out_amount: u64,
    pub slippage_bps: u64,
    pub platform_fee_bps: u64,
}

pub async fn analyze_trade(transaction: &DexTransaction) -> Option<DexTrade> {
    // Parse Jupiter instruction
    let swap_params = parse_jupiter_swap(transaction)?;

    let trade_type = determine_trade_type(
        transaction.token_in?,
        transaction.token_out?,
        swap_params.in_amount,
        swap_params.out_amount
    );

    Some(DexTrade {
        protocol: DexProtocol::Jupiter,
        trade_type,
        signature: transaction.signature.clone(),
        timestamp: transaction.block_time?,
        slippage: swap_params.slippage_bps as f64 / 10000.0,
        price_impact: calculate_price_impact(swap_params),
    })
}

fn parse_jupiter_swap(transaction: &DexTransaction) -> Option<JupiterSwapParams> {
    let data = transaction.instruction_data.as_slice();

    // Jupiter v6 instruction layout
    match data.get(0)? {
        // Swap instruction discriminator
        2 => {
            Some(JupiterSwapParams {
                in_amount: u64::from_le_bytes(data[1..9].try_into().ok()?),
                out_amount: u64::from_le_bytes(data[9..17].try_into().ok()?),
                slippage_bps: u64::from_le_bytes(data[17..25].try_into().ok()?),
                platform_fee_bps: u64::from_le_bytes(data[25..33].try_into().ok()?),
            })
        },
        _ => None,
    }
}