use reqwest;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

pub struct JupiterApiClient {
    base_url: String,
    http_client: reqwest::Client,
}

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