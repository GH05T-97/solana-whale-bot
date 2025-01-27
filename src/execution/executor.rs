use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    transaction::Transaction,
    signature::Signature,
    pubkey::Pubkey,
    signer::Signer,
    signer::keypair::Keypair,
};
use std::time::{Duration, Instant};
use thiserror::Error;
use log::{error, info, warn};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::execution::clients::{
    JupiterApiClient,
    RaydiumApiClient,
};

use crate::strategy::types::{TradeSignal, TradeDirection};
use crate::execution::types::{
    OrderRequest,
    OrderResult,
    SwapParams,
    DexType,
    OrderDirection,
    OrderType,
    TimeInForce,
    OrderStatus,
};

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

use crate::execution::{
    ExecutionError,
    RetryHandler,
    retry::RetryConfig
};

use crate::SolanaConfig;

// USDC mint address on mainnet
const USDC_MINT: Pubkey = Pubkey::new_from_array([
    70, 144, 236, 149, 124, 73, 90, 105,
    103, 53, 184, 24, 197, 42, 68, 252,
    197, 24, 51, 71, 182, 102, 126, 148,
    161, 255, 242, 163, 107, 120, 37, 107
]);

// SOL mint address
const SOL_MINT: Pubkey = Pubkey::new_from_array([
    6, 155, 136, 87, 254, 171, 129, 132,
    251, 104, 127, 99, 70, 24, 192, 53,
    218, 196, 57, 220, 26, 235, 59, 85,
    153, 162, 174, 237, 137, 133, 151, 96
]);

#[derive(Debug)]
pub struct Position {
    pub token_mint: Pubkey,
    pub amount: u64,
}

#[derive(Clone, Debug)]
pub struct TokenAvailability {
    pub jupiter_available: bool,
    pub raydium_available: bool,
}

// Remove Default derive since RpcClient and Keypair don't implement it
#[derive(Debug)]
pub struct TradeExecutor {
    solana_config: SolanaConfig,
    client: Arc<RpcClient>, // Wrap in Arc to make it Clone
    orders: Arc<RwLock<HashMap<String, OrderRequest>>>,
    active_positions: Arc<RwLock<HashMap<Pubkey, Position>>>,
    keypair: Arc<Keypair>, // Wrap in Arc to make it Clone
    retry_handler: RetryHandler,
    token_availability_cache: Arc<RwLock<HashMap<Pubkey, TokenAvailability>>>,
    jupiter_client: JupiterApiClient,
    raydium_client: RaydiumApiClient,
}

impl Clone for TradeExecutor {
    fn clone(&self) -> Self {
        Self {
            solana_config: self.solana_config.clone(),
            client: self.client.clone(),
            orders: self.orders.clone(),
            active_positions: self.active_positions.clone(),
            keypair: self.keypair.clone(),
            retry_handler: self.retry_handler.clone(),
            token_availability_cache: self.token_availability_cache.clone(),
            jupiter_client: self.jupiter_client.clone(),
            raydium_client: self.raydium_client.clone(),
        }
    }
}

impl TradeExecutor {
    pub fn new(solana_config: SolanaConfig) -> Self {
        Self {
            client: Arc::new(solana_config.create_rpc_client()),
            solana_config: solana_config.clone(),
            orders: Arc::new(RwLock::new(HashMap::new())),
            active_positions: Arc::new(RwLock::new(HashMap::new())),
            keypair: Arc::new(solana_config.keypair), // Don't clone Keypair, wrap in Arc
            retry_handler: RetryHandler::new(RetryConfig::default()),
            token_availability_cache: Arc::new(RwLock::new(HashMap::new())),
            jupiter_client: JupiterApiClient::new(),
            raydium_client: RaydiumApiClient::new(),
        }
    }

    // ... rest of the implementation remains the same until create_jupiter_transaction

    async fn create_jupiter_transaction(&self, order_request: &OrderRequest) -> Result<Transaction, ExecutionError> {
        let swap_params = SwapParams {
            input_mint: USDC_MINT,
            output_mint: order_request.token,
            amount: order_request.size.to_u64().ok_or_else(||
                ExecutionError::ValidationFailed("Failed to convert size to u64".to_string())
            )?,
        };

        let swap_instruction = self.jupiter_client.get_swap_instruction(&swap_params).await?;

        let recent_blockhash = self.client
            .get_latest_blockhash()
            .map_err(|e| ExecutionError::BlockhashFetchFailed(e.to_string()))?;

        Ok(Transaction::new_signed_with_payer(
            &[swap_instruction],
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_blockhash
        ))
    }

    async fn create_raydium_transaction(&self, order_request: &OrderRequest) -> Result<Transaction, ExecutionError> {
        let swap_params = RaydiumSwapParams {
            pool_id: SOL_MINT,
            amount_in: order_request.size.to_u64().ok_or_else(||
                ExecutionError::ValidationFailed("Failed to convert size to u64".to_string())
            )?,
            min_amount_out: self.calculate_min_output_amount(order_request)?,
        };

        let swap_instruction = self.raydium_client.get_swap_instruction(&swap_params).await?;

        let recent_blockhash = self.client
            .get_latest_blockhash()
            .map_err(|e| ExecutionError::BlockhashFetchFailed(e.to_string()))?;

        Ok(Transaction::new_signed_with_payer(
            &[swap_instruction],
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_blockhash
        ))
    }

    fn calculate_min_output_amount(&self, request: &OrderRequest) -> Result<u64, ExecutionError> {
        let slippage_factor = Decimal::new(99, 2); // 0.99 as Decimal
        let min_amount = request.size * slippage_factor;
        min_amount.to_u64().ok_or_else(||
            ExecutionError::ValidationFailed("Failed to convert minimum amount to u64".to_string())
        )
    }

    async fn validate_order(&self, order_request: &OrderRequest) -> Result<(), ExecutionError> {
        if order_request.size == Decimal::zero() {
            return Err(ExecutionError::ValidationFailed(
                "Trade size cannot be zero".to_string()
            ));
        }

        Ok(())
    }

    // ... rest of the implementation remains the same
}