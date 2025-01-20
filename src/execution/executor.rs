use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    transaction::Transaction,
    signature::Signature,
    pubkey::Pubkey,
    signer::Signer,
};
use std::time::{Duration, Instant};
use thiserror::Error;
use log::{error, info, warn};

use crate::execution::{
    JupiterApiClient,
    RaydiumApiClient,
};

pub struct TradeExecutor {
    client: RpcClient,
    orders: Arc<RwLock<HashMap<String, OrderRequest>>>,
    active_positions: Arc<RwLock<HashMap<Pubkey, Position>>>,
    keypair: Keypair,
    config: ExecutorConfig,
    retry_handler: RetryHandler,
    token_availability_cache: Arc<RwLock<HashMap<Pubkey, TokenAvailability>>>,
    jupiter_client: JupiterApiClient,
    raydium_client: RaydiumApiClient,
}

impl TradeExecutor {
    pub fn new(keypair: Keypair, config: ExecutorConfig) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                "https://api.mainnet-beta.solana.com".to_string(),
                CommitmentConfig::confirmed(),
            ),
            orders: Arc::new(RwLock::new(HashMap::new())),
            active_positions: Arc::new(RwLock::new(HashMap::new())),
            keypair,
            config,
            retry_handler: RetryHandler::new(RetryConfig::default()),
            token_availability_cache: Arc::new(RwLock::new(HashMap::new())),
            jupiter_client: JupiterApiClient::new(),
            raydium_client: RaydiumApiClient::new(),
        }
    }

    async fn check_token_availability(&self, token_mint: &Pubkey) -> Result<TokenAvailability, ExecutionError> {
        // Check cache first
        {
            let cache = self.token_availability_cache.read().await;
            if let Some(availability) = cache.get(token_mint) {
                return Ok(availability.clone());
            }
        }

        // If not in cache, perform availability checks
        let jupiter_available = self.check_jupiter_token_availability(token_mint).await?;
        let raydium_available = self.check_raydium_token_availability(token_mint).await?;

        let availability = TokenAvailability {
            jupiter_available,
            raydium_available,
        };

        // Update cache
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
        match self.raydium_client.get_liquidity(*token_mint).await {
            Ok(liquidity) if liquidity.total_liquidity > 0 => Ok(true),
            _ => Ok(false)
        }
    }

    // Determine best DEX for trading
    async fn select_best_dex(&self, token_mint: &Pubkey) -> Result<DexType, ExecutionError> {
        let availability = self.check_token_availability(token_mint).await?;

        // Prioritization logic
        match (availability.jupiter_available, availability.raydium_available) {
            (true, true) => Ok(DexType::Jupiter),  // Prefer Jupiter by default
            (true, false) => Ok(DexType::Jupiter),
            (false, true) => Ok(DexType::Raydium),
            (false, false) => Err(ExecutionError::TokenNotAvailable(
                "Token not available on either Jupiter or Raydium".to_string()
            ))
        }
    }

    // Execute trade method
    pub async fn execute_trade(&self, signal: TradeSignal) -> Result<OrderResult, ExecutionError> {
        // Determine which DEX has the token available
        let dex_type = self.select_best_dex(&signal.token).await?;

        // Prepare order request
        let mut order_request = self.prepare_order_request(signal).await?;
        order_request.dex_type = Some(dex_type);

        // Validate order
        self.validate_order(&order_request).await?;

        // Execute trade based on available DEX
        match dex_type {
            DexType::Jupiter => self.execute_jupiter_trade(&order_request).await,
            DexType::Raydium => self.execute_raydium_trade(&order_request).await,
        }
    }

    // Execute trade on Jupiter
    async fn execute_jupiter_trade(&self, order_request: &OrderRequest) -> Result<OrderResult, ExecutionError> {
        let jupiter_client = JupiterSwapClient::new();

        let swap_params = SwapParams {
            input_mint: USDC_MINT, // or determine input token
            output_mint: order_request.output_token,
            amount: order_request.amount,
            slippage_bps: 50, // 0.5% slippage
            user_public_key: self.keypair.pubkey(),
        };

        // Execute swap
        let swap_result = jupiter_client.get_swap_instruction(&swap_params).await?;

        // Create and return order result
        Ok(OrderResult {
            order_id: "jupiter_trade".to_string(),
            status: OrderStatus::Filled,
            // Add additional details as needed
        })
    }

    // Execute trade on Raydium
    async fn execute_raydium_trade(&self, order_request: &OrderRequest) -> Result<OrderResult, ExecutionError> {
        let raydium_client = RaydiumSwapClient::new();

        let swap_params = RaydiumSwapParams {
            input_token: SOL_MINT, // or determine input token
            output_token: order_request.output_token,
            input_amount: order_request.amount,
            min_output_amount: self.calculate_min_output_amount(order_request),
            user_public_key: self.keypair.pubkey(),
        };

        // Execute swap
        let swap_result = raydium_client.get_swap_instruction(&swap_params).await?;

        // Create and return order result
        Ok(OrderResult {
            order_id: "raydium_trade".to_string(),
            status: OrderStatus::Filled,
            // Add additional details as needed
        })
    }

    // Helper method to calculate minimum output amount
    fn calculate_min_output_amount(&self, request: &OrderRequest) -> u64 {
        // Apply slippage protection
        let slippage_factor = 0.99; // 1% slippage tolerance
        (request.amount as f64 * slippage_factor) as u64
    }

    // Validate order parameters
    async fn validate_order(&self, order_request: &OrderRequest) -> Result<(), ExecutionError> {
        // Check basic order validation
        if order_request.amount == 0 {
            return Err(ExecutionError::ValidationError(
                "Trade amount cannot be zero".to_string()
            ));
        }

        // Additional validation can be added here
        Ok(())
    }

    // Prepare order request from trade signal
    async fn prepare_order_request(&self, signal: TradeSignal) -> Result<OrderRequest, ExecutionError> {
        Ok(OrderRequest {
            input_token: USDC_MINT, // or determine dynamically
            output_token: signal.token,
            amount: signal.size.to_u64().ok_or(ExecutionError::ValidationError(
                "Invalid trade size".to_string()
            ))?,
            dex_type: None, // Will be set later
            order_type: OrderType::Market,
            stop_loss: signal.stop_loss.to_u64().ok_or(ExecutionError::ValidationError(
                "Invalid stop loss".to_string()
            ))?,
            take_profit: signal.take_profit.to_u64().ok_or(ExecutionError::ValidationError(
                "Invalid take profit".to_string()
            ))?,
        })
    }
}