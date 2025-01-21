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

use crate::execution::clients::{
    JupiterApiClient,
    RaydiumApiClient,
};

use crate::strategy::types::TradeSignal;
use super::types::DexType;

use crate::execution::types::{
    OrderRequest,
    OrderResult,
    SwapParams,
};

use crate::SolanaConfig;

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
            keypair: solana_config.keypair.clone(), // Use keypair from SolanaConfig
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

    async fn submit_transaction(&self, transaction: Transaction) -> Result<Signature, ExecutionError> {
        self.client
            .send_and_confirm_transaction(&transaction)
            .await
            .map_err(|e| ExecutionError::TransactionSubmissionError(e.to_string()))
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

        // Create transaction based on DEX
        let transaction = match dex_type {
            DexType::Jupiter => self.create_jupiter_transaction(&order_request).await?,
            DexType::Raydium => self.create_raydium_transaction(&order_request).await?,
        };

        // Submit transaction to Solana network
        let signature = self.submit_transaction(transaction).await?;

        // Return order result
        Ok(OrderResult {
            order_id: signature.to_string(),
            status: OrderStatus::Filled,
            fills: Vec::new(), // Empty vector
            average_price: 0.00, // Use None for Optional<Decimal>
            timestamp: Utc::now(), // Use Utc::now() instead of DateTime::now()
        })
    }

    // Execute trade on Jupiter
    // Create transaction for Jupiter swap
    async fn create_jupiter_transaction(&self, order_request: &OrderRequest) -> Result<Transaction, ExecutionError> {
        let jupiter_client = JupiterSwapClient::new();

        // Prepare swap parameters
        let swap_params = SwapParams {
            input_mint: USDC_MINT,
            output_mint: order_request.output_token,
            amount: order_request.amount,
        };

        // Get swap instruction
        let swap_instruction = jupiter_client.get_swap_instruction(&swap_params).await?;

        // Get recent blockhash
        let recent_blockhash = self.client.get_latest_blockhash()
            .await
            .map_err(|e| ExecutionError::BlockhashError(e.to_string()))?;

        // Create and sign transaction
        Ok(Transaction::new_signed_with_payer(
            &[swap_instruction],
            Some(&self.keypair.pubkey()),
            &[&self.keypair],
            recent_blockhash
        ))
    }

    // Execute trade on Raydium
   // Similar method for Raydium transaction creation
   async fn create_raydium_transaction(&self, order_request: &OrderRequest) -> Result<Transaction, ExecutionError> {
    let raydium_client = RaydiumSwapClient::new();

    // Prepare swap parameters
    let swap_params = RaydiumSwapParams {
        input_token: SOL_MINT,
        output_token: order_request.output_token,
        input_amount: order_request.amount,
        min_output_amount: self.calculate_min_output_amount(order_request),
        user_public_key: self.keypair.pubkey(),
    };

    // Get swap instruction
    let swap_instruction = raydium_client.get_swap_instruction(&swap_params).await?;

    // Get recent blockhash
    let recent_blockhash = self.client.get_latest_blockhash()
        .await
        .map_err(|e| ExecutionError::BlockhashError(e.to_string()))?;

    // Create and sign transaction
    Ok(Transaction::new_signed_with_payer(
        &[swap_instruction],
        Some(&self.keypair.pubkey()),
        &[&self.keypair],
        recent_blockhash
    ))
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
            token: signal.token, // Use the token from trade signal
            direction: match signal.direction {
                TradeDirection::Long => OrderDirection::Buy,
                TradeDirection::Short => OrderDirection::Sell,
            },
            size: signal.size,
            price: signal.entry_price, // Use entry price from trade signal
            order_type: OrderType::Market, // Or determine based on signal
            time_in_force: TimeInForce::GoodTilCancelled, // Default or from configuration
            stop_loss: Some(signal.stop_loss),
            take_profit: Some(signal.take_profit),
        })
    }
}