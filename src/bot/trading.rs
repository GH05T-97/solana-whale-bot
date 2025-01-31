use std::sync::Arc;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::{commitment_config::CommitmentConfig, account::Account};
use solana_transaction_status::{
    option_serializer::OptionSerializer,
    UiTransactionEncoding,
    UiTransactionTokenBalance,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Clone)]  // Derive Clone directly
pub struct TradingVolume {
    pub token_name: String,
    pub total_volume: f64,
    pub trade_count: u32,
    pub average_trade_size: f64,
    last_update: SystemTime,
}

pub struct VolumeTracker {
    rpc_client: Arc<RpcClient>,
    pub min_volume: f64,
    pub max_volume: f64,
    volume_data: HashMap<String, TradingVolume>,
    time_window: Duration,
    token_names_cache: HashMap<String, String>,
}

#[derive(Deserialize)]
struct RaydiumPriceResponse {
    data: HashMap<String, TokenPrice>,
}

#[derive(Deserialize)]
struct TokenPrice {
    price: f64,
    #[serde(rename = "mint")]
}

impl VolumeTracker {
    pub fn new(rpc_url: &str, min_volume: f64, max_volume: f64) -> Self {
        Self {
            rpc_client: Arc::new(RpcClient::new(rpc_url.to_string())),
            min_volume,
            max_volume,
            volume_data: HashMap::new(),
            time_window: Duration::from_secs(900),
            token_names_cache: HashMap::new(),
        }
    }

    pub async fn track_trades(&mut self) -> Result<Vec<TradingVolume>, Box<dyn std::error::Error + Send + Sync>> {
        let dex_program_id = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")?;

        let signatures = self.rpc_client.get_signatures_for_address(&dex_program_id)?;
        let mut hot_volumes = Vec::new();

        for sig_info in signatures {
            let tx = self.rpc_client.get_transaction_with_config(
                &sig_info.signature.parse()?,
                RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Json),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                },
            )?;

            if let Some(meta) = tx.transaction.meta {
                if let Some(token_balances) = <OptionSerializer<Vec<UiTransactionTokenBalance>> as Into<Option<Vec<UiTransactionTokenBalance>>>>::into(meta.pre_token_balances) {
                    for (pre, post) in token_balances.iter().zip(meta.post_token_balances.unwrap()) {
                        let amount_change = (post.ui_token_amount.ui_amount.unwrap_or(0.0)
                            - pre.ui_token_amount.ui_amount.unwrap_or(0.0)).abs();

                        let token_price = self.get_token_price(&post.mint).await?;
                        let trade_value = amount_change * token_price;

                        if trade_value >= self.min_volume && trade_value <= self.max_volume {
                            let token_name = self.get_token_name(&post.mint).await?;

                            if let Some(existing) = self.volume_data.get_mut(&post.mint) {
                                existing.total_volume += trade_value;
                                existing.trade_count += 1;
                                existing.average_trade_size = existing.total_volume / existing.trade_count as f64;
                                existing.last_update = SystemTime::now();

                                if existing.trade_count >= 3 {
                                    hot_volumes.push(existing.clone());
                                }
                            } else {
                                let volume = TradingVolume {
                                    token_address: post.mint.clone(),
                                    token_name,
                                    total_volume: trade_value,
                                    trade_count: 1,
                                    average_trade_size: trade_value,
                                    last_update: SystemTime::now(),
                                };
                                self.volume_data.insert(post.mint.clone(), volume);
                            }
                        }
                    }
                }
            }
        }

        self.clean_old_data();
        Ok(hot_volumes)
    }

    pub fn get_hot_pairs(&self) -> Vec<TradingVolume> {  // Return owned TradingVolume instead of references
        self.volume_data
            .values()
            .filter(|v| {
                v.average_trade_size >= self.min_volume
                && v.average_trade_size <= self.max_volume
                && v.trade_count >= 3
            })
            .cloned()  // Clone the filtered values
            .collect()
    }

    async fn get_token_price(&self, mint: &str) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://api.raydium.io/v2/main/price?tokens={}",
            mint
        );

        let client = reqwest::Client::new();
        let response = client.get(&url)
            .send()
            .await?
            .json::<RaydiumPriceResponse>()
            .await?;

        if let Some(token_data) = response.data.get(mint) {
            Ok(token_data.price)
        } else {
            Err("Price not found on Raydium".into())
        }
    }

    fn clean_old_data(&mut self) {
        let now = SystemTime::now();
        self.volume_data.retain(|_, v| {
            if let Ok(duration) = now.duration_since(v.last_update) {
                duration < self.time_window
            } else {
                false
            }
        });
    }

    async fn get_token_name(&self, mint: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(name) = self.token_names_cache.get(mint) {
            return Ok(name.clone());
        }

        // For now, just return the mint address as the name
        // You can implement proper metadata fetching later
        Ok(mint.to_string())
    }
}