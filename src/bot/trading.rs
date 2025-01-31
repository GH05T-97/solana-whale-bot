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

const RAYDIUM_DEX_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const RAYDIUM_AMM_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

#[derive(Debug, Deserialize)]
struct RaydiumPriceResponse {
    data: HashMap<String, TokenPrice>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct TokenPrice {
    price: f64,
    #[serde(rename = "mint")]
    token_mint: String,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct TradingVolume {
    token_address: String,
    pub token_name: String,
    pub total_volume: f64,
    pub trade_count: u32,
    pub swap_count: u32,  // New field to track AMM swaps
    pub average_trade_size: f64,
    last_update: SystemTime,
}

#[allow(dead_code)]
pub struct VolumeTracker {
    rpc_client: Arc<RpcClient>,
    pub min_volume: f64,
    pub max_volume: f64,
    volume_data: HashMap<String, TradingVolume>,
    time_window: Duration,
    token_names_cache: HashMap<String, String>,
    price_cache: HashMap<String, (f64, SystemTime)>,
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
            price_cache: HashMap::new(),
        }
    }

    pub async fn track_trades(&mut self) -> Result<Vec<TradingVolume>, Box<dyn std::error::Error + Send + Sync>> {
        let mut all_volumes = Vec::new();

        // Track DEX trades
        let dex_volumes = self.track_dex_trades().await?;
        all_volumes.extend(dex_volumes);

        // Track AMM swaps
        let swap_volumes = self.track_amm_swaps().await?;

        // Merge swap volumes with existing volumes
        for swap_vol in swap_volumes {
            if let Some(existing) = all_volumes.iter_mut().find(|v| v.token_address == swap_vol.token_address) {
                existing.total_volume += swap_vol.total_volume;
                existing.swap_count += swap_vol.swap_count;
                existing.average_trade_size = existing.total_volume /
                    (existing.trade_count as f64 + existing.swap_count as f64);
            } else {
                all_volumes.push(swap_vol);
            }
        }

        self.clean_old_data();
        Ok(all_volumes)
    }

    async fn track_dex_trades(&self) -> Result<Vec<TradingVolume>, Box<dyn std::error::Error + Send + Sync>> {
        let dex_program_id = Pubkey::from_str(RAYDIUM_DEX_PROGRAM)?;
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
                    self.process_token_balances(&token_balances, meta.post_token_balances.unwrap(), &mut hot_volumes).await?;
                }
            }
        }

        Ok(hot_volumes)
    }

    async fn track_amm_swaps(&self) -> Result<Vec<TradingVolume>, Box<dyn std::error::Error + Send + Sync>> {
        let amm_program_id = Pubkey::from_str(RAYDIUM_AMM_PROGRAM)?;
        let signatures = self.rpc_client.get_signatures_for_address(&amm_program_id)?;
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
                    self.process_token_balances(&token_balances, meta.post_token_balances.unwrap(), &mut hot_volumes).await?;
                }
            }
        }

        Ok(hot_volumes)
    }

    async fn process_token_balances(
        &self,
        pre_balances: &[UiTransactionTokenBalance],
        post_balances: Vec<UiTransactionTokenBalance>,
        hot_volumes: &mut Vec<TradingVolume>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for (pre, post) in pre_balances.iter().zip(post_balances) {
            let amount_change = (post.ui_token_amount.ui_amount.unwrap_or(0.0)
                - pre.ui_token_amount.ui_amount.unwrap_or(0.0)).abs();

            let token_price = self.get_token_price(&post.mint).await?;
            let trade_value = amount_change * token_price;

            if trade_value >= self.min_volume && trade_value <= self.max_volume {
                let token_name = self.get_token_name(&post.mint).await?;

                if let Some(existing) = hot_volumes.iter_mut().find(|v| v.token_address == post.mint) {
                    existing.total_volume += trade_value;
                    existing.trade_count += 1;
                    existing.average_trade_size = existing.total_volume /
                        (existing.trade_count as f64 + existing.swap_count as f64);
                    existing.last_update = SystemTime::now();
                } else {
                    hot_volumes.push(TradingVolume {
                        token_address: post.mint.clone(),
                        token_name,
                        total_volume: trade_value,
                        trade_count: 1,
                        swap_count: 0,
                        average_trade_size: trade_value,
                        last_update: SystemTime::now(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn get_hot_pairs(&self) -> Vec<TradingVolume> {
        self.volume_data
            .values()
            .filter(|v| {
                v.average_trade_size >= self.min_volume
                && v.average_trade_size <= self.max_volume
                && (v.trade_count + v.swap_count) >= 3  // Consider both trades and swaps
            })
            .cloned()
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