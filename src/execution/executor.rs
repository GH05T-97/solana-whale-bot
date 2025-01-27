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

// Common imports
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

const USDC_MINT: Pubkey = Pubkey::new_from_array([/* your USDC mint bytes */]); // Replace with actual USDC mint bytes
const SOL_MINT: Pubkey = Pubkey::new_from_array([/* your SOL mint bytes */]); // Add SOL mint constant

pub struct JupiterSwapParams {
    pub in_amount: u64,
    pub out_amount: u64,
    pub slippage_bps: u64,
    pub platform_fee_bps: u64,
}

pub struct RaydiumSwapParams {
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub pool_id: Pubkey,
}

pub struct Position {
    pub token_mint: Pubkey,
    pub amount: u64,
}

#[derive(Clone)]
pub struct TokenAvailability {
    pub jupiter_available: bool,
    pub raydium_available: bool,
}

#[derive(Clone, Debug, Default)]
pub struct TradeExecutor {
    solana_config: SolanaConfig,
    client: RpcClient,
    orders: Arc<RwLock<HashMap<String, OrderRequest>>>,
    active_positions: Arc<RwLock<HashMap<Pubkey, Position>>>,
    keypair: Keypair,
    retry_handler: RetryHandler,
    token_availability_cache: Arc<RwLock<HashMap<Pubkey, TokenAvailability>>>,
    jupiter_client: JupiterApiClient,
    raydium_client: RaydiumApiClient,
}

impl TradeExecutor {
    pub fn new(solana_config: SolanaConfig) -> Self {
        Self {
            client: solana_config.create_rpc_client(),
            solana_config: solana_config.clone(),
            orders: Arc::new(RwLock::new(HashMap::new())),
            active_positions: Arc::new(RwLock::new(HashMap::new())),
            keypair: solana_config.keypair.clone(),
            retry_handler: RetryHandler::new(RetryConfig::default()),
            token_availability_cache: Arc::new(RwLock::new(HashMap::new())),
            jupiter_client: JupiterApiClient::new(),
            raydium_client: RaydiumApiClient::new(),
        }
    }

    async fn check_token_availability(&self, token_mint: &Pubkey) -> Result<TokenAvailability, ExecutionError> {
        {
            let cache = self.token_availability_cache.read().await;
            if let Some(availability) = cache.get(token_mint) {
                return Ok(availability.clone());
            }
        }

        let jupiter_available = self.check_jupiter_token_availability(token_mint).await?;
        let raydium_available = self.check_raydium_token_availability(token_mint).await?;

        let availability = TokenAvailability {
            jupiter_available,
            raydium_available,
        };

        {
            let mut cache = self.token_availability_cache.write().await;
            cache.insert(*token_mint, availability.clone());
        }

        Ok(availability)
    }

    async fn check_jupiter_token_availability(&self, token_mint: &Pubkey) -> Result<bool, ExecutionError> {
        match self.jupiter_client.get_quote(
            *token_mint,
            USDC_MINT,
            1_000_000
        ).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false)
        }
    }

    async fn check_raydium_token_availability(&self, token_mint: &Pubkey) -> Result<bool, ExecutionError> {
        match self.raydium_client.get_liquidity_pool(*token_mint).await {
            Ok(liquidity) if liquidity.total_liquidity > 0 => Ok(true),
            _ => Ok(false)
        }
    }

    async fn select_best_dex(&self, token_mint: &Pubkey) -> Result<DexType, ExecutionError> {
        let availability = self.check_token_availability(token_mint).await?;

        match (availability.jupiter_available, availability.raydium_available) {
            (true, true) => Ok(DexType::Jupiter),
            (true, false) => Ok(DexType::Jupiter),
            (false, true) => Ok(DexType::Raydium),
            (false, false) => Err(ExecutionError::NoLiquidityAvailable(
                "Token not available on either Jupiter or Raydium".to_string()
            ))
        }
    }

    async fn submit_transaction(&self, transaction: Transaction) -> Result<Signature, ExecutionError> {
        self.client
            .send_and_confirm_transaction_with_spinner(&transaction)
            .map_err(|e| ExecutionError::TransactionFailed(e.to_string()))
    }

    pub async fn execute_trade(&self, signal: TradeSignal) -> Result<OrderResult, ExecutionError> {
        let dex_type = self.select_best_dex(&signal.token).await?;
        let order_request = self.prepare_order_request(signal).await?;

        self.validate_order(&order_request).await?;

        let transaction = match dex_type {
            DexType::Jupiter => self.create_jupiter_transaction(&order_request).await?,
            DexType::Raydium => self.create_raydium_transaction(&order_request).await?,
        };

        let signature = self.submit_transaction(transaction).await?;

        Ok(OrderResult {
            order_id: signature.to_string(),
            status: OrderStatus::Filled,
            fills: Vec::new(),
            average_price: None,
            timestamp: Utc::now(),
        })
    }

    async fn create_jupiter_transaction(&self, order_request: &OrderRequest) -> Result<Transaction, ExecutionError> {
        let swap_params = SwapParams {
            input_mint: USDC_MINT,
            output_mint: order_request.token,
            amount: order_request.size,
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
            amount_in: order_request.size,
            min_amount_out: self.calculate_min_output_amount(order_request),
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

    fn calculate_min_output_amount(&self, request: &OrderRequest) -> u64 {
        let slippage_factor = Decimal::new(99, 2); // 0.99 as Decimal
        let min_amount = request.size * slippage_factor;
        min_amount.to_u64().unwrap_or(0)
    }

    async fn validate_order(&self, order_request: &OrderRequest) -> Result<(), ExecutionError> {
        if order_request.size == 0 {
            return Err(ExecutionError::ValidationFailed(
                "Trade size cannot be zero".to_string()
            ));
        }

        Ok(())
    }

    async fn prepare_order_request(&self, signal: TradeSignal) -> Result<OrderRequest, ExecutionError> {
        Ok(OrderRequest {
            token: signal.token,
            direction: match signal.direction {
                TradeDirection::Long => OrderDirection::Buy,
                TradeDirection::Short => OrderDirection::Sell,
            },
            size: signal.size,
            price: signal.entry_price,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::GoodTilCancelled,
            stop_loss: signal.stop_loss,
            take_profit: signal.take_profit,
        })
    }
}