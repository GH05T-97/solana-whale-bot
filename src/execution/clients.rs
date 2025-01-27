use reqwest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::{
    pubkey::Pubkey,
    instruction::Instruction,
};
use chrono::{DateTime, Utc};
use thiserror::Error;

// Add missing type definitions
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapParams {
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub amount: u64,
    pub slippage_bps: u64,
    pub user_public_key: Pubkey,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u64,
    pub user_public_key: String,
    pub route_plan: Vec<RoutePlan>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoutePlan {
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapResponse {
    pub program_id: Pubkey,
    pub instruction_data: Vec<u8>,
    pub accounts: Vec<AccountMeta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Quote {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: u64,
    pub out_amount: u64,
    pub price_impact: f64,
    pub route_plan: Vec<RoutePlan>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Liquidity {
    pub pool_id: String,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub total_liquidity: u64,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("API error: {0}")]
    ApiError(String),
}

#[derive(Clone, Debug, Default)]
pub struct JupiterApiClient {
    base_url: String,
    http_client: reqwest::Client,
}

#[derive(Clone, Debug, Default)]
pub struct RaydiumApiClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl JupiterApiClient {
    pub fn new() -> Self {
        Self {
            base_url: "https://quote-api.jup.ag/v4".to_string(),
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn get_swap_instruction(
        &self,
        params: &SwapParams
    ) -> Result<Instruction, ApiError> {
        // Get swap quote first
        let quote = self.get_quote(
            params.input_mint,
            params.output_mint,
            params.amount
        ).await?;

        // Construct swap route
        let swap_route_url = format!("{}/swap", self.base_url);

        // Prepare swap payload
        let swap_payload = SwapRequest {
            input_mint: params.input_mint.to_string(),
            output_mint: params.output_mint.to_string(),
            amount: params.amount,
            slippage_bps: params.slippage_bps,
            user_public_key: params.user_public_key.to_string(),
            route_plan: quote.route_plan,
        };

        // Send swap request to get instruction
        let swap_response = self.http_client
            .post(&swap_route_url)
            .json(&swap_payload)
            .send()
            .await?
            .json::<SwapResponse>()
            .await?;

        Ok(Instruction::new_with_bytes(
            swap_response.program_id,
            &swap_response.instruction_data,
            swap_response.accounts,
        ))
    }

    pub async fn get_quote(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64,
    ) -> Result<Quote, ApiError> {
        let url = format!("{}/quote", self.base_url);

        let response = self.http_client
            .get(&url)
            .query(&[
                ("inputMint", input_mint.to_string()),
                ("outputMint", output_mint.to_string()),
                ("amount", amount.to_string()),
            ])
            .send()
            .await?
            .json::<Quote>()
            .await?;

        Ok(response)
    }
}

impl RaydiumApiClient {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.raydium.io/v2".to_string(),
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn get_swap_instruction(
        &self,
        params: &RaydiumSwapParams
    ) -> Result<Instruction, ApiError> {
        let pool_info = self.get_liquidity(params.pool_id).await?;

        let swap_route_url = format!("{}/swap", self.base_url);

        let swap_payload = RaydiumSwapRequest {
            input_token: params.amount_in.to_string(),
            output_token: params.min_amount_out.to_string(),
            input_amount: params.amount_in,
            min_output_amount: params.min_amount_out,
            pool_id: pool_info.pool_id,
        };

        let swap_response = self.http_client
            .post(&swap_route_url)
            .json(&swap_payload)
            .send()
            .await?
            .json::<SwapResponse>()
            .await?;

        Ok(Instruction::new_with_bytes(
            swap_response.program_id,
            &swap_response.instruction_data,
            swap_response.accounts,
        ))
    }

    pub async fn get_liquidity(&self, token_mint: Pubkey) -> Result<Liquidity, ApiError> {
        let url = format!("{}/liquidity/{}", self.base_url, token_mint);

        let response = self.http_client
            .get(&url)
            .send()
            .await?
            .json::<Liquidity>()
            .await?;

        Ok(response)
    }
}