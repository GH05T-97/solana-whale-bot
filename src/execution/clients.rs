use reqwest;
use serde::{Deserialize, Serialize};
// Common imports to add
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};

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

        // Convert to Solana instruction
        Ok(Instruction::new_with_bytes(
            swap_response.program_id,
            &swap_response.instruction_data,
            swap_response.accounts
        ))
    }

    pub async fn get_quote(&self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64
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
        // Get liquidity pool information
        let pool_info = self.get_liquidity_pool(
            params.input_token,
            params.output_token
        ).await?;

        // Construct swap route URL
        let swap_route_url = format!("{}/swap", self.base_url);

        // Prepare swap payload
        let swap_payload = RaydiumSwapRequest {
            input_token: params.input_token.to_string(),
            output_token: params.output_token.to_string(),
            input_amount: params.input_amount,
            min_output_amount: params.min_output_amount,
            user_public_key: params.user_public_key.to_string(),
            pool_id: pool_info.pool_id,
        };

        // Send swap request to get instruction
        let swap_response = self.http_client
            .post(&swap_route_url)
            .json(&swap_payload)
            .send()
            .await?
            .json::<SwapResponse>()
            .await?;

        // Convert to Solana instruction
        Ok(Instruction::new_with_bytes(
            swap_response.program_id,
            &swap_response.instruction_data,
            swap_response.accounts
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