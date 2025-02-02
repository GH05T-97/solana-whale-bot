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
use log::{info, warn, error};

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
        info!("Initializing VolumeTracker with min_volume: ${}, max_volume: ${}", min_volume, max_volume);
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
        info!("Starting trade tracking cycle");
        let mut all_volumes = Vec::new();

        info!("Tracking DEX trades...");
        let dex_volumes = self.track_dex_trades().await?;
        info!("Found {} DEX trading volumes", dex_volumes.len());
        all_volumes.extend(dex_volumes);

        info!("Tracking AMM swaps...");
        let swap_volumes = self.track_amm_swaps().await?;
        info!("Found {} AMM swap volumes", swap_volumes.len());

        // Merge swap volumes with existing volumes
        info!("Merging DEX and AMM volumes...");
        for swap_vol in swap_volumes {
            if let Some(existing) = all_volumes.iter_mut().find(|v| v.token_address == swap_vol.token_address) {
                info!("Merging volumes for token {}: Adding ${:.2} from swaps",
                    swap_vol.token_name, swap_vol.total_volume);
                existing.total_volume += swap_vol.total_volume;
                existing.swap_count += swap_vol.swap_count;
                existing.average_trade_size = existing.total_volume /
                    (existing.trade_count as f64 + existing.swap_count as f64);
            } else {
                info!("Adding new swap volume for token {}: ${:.2}",
                    swap_vol.token_name, swap_vol.total_volume);
                all_volumes.push(swap_vol);
            }
        }

        self.clean_old_data();
        info!("Completed trade tracking cycle. Found {} total volumes", all_volumes.len());
        Ok(all_volumes)
    }

    async fn track_dex_trades(&self) -> Result<Vec<TradingVolume>, Box<dyn std::error::Error + Send + Sync>> {
        let dex_program_id = Pubkey::from_str(RAYDIUM_DEX_PROGRAM)?;
        info!("Fetching signatures for DEX program {}", RAYDIUM_DEX_PROGRAM);
        let signatures = self.rpc_client.get_signatures_for_address(&dex_program_id)?;
        info!("Found {} DEX transactions to analyze", signatures.len());
        let mut hot_volumes = Vec::new();

        for (i, sig_info) in signatures.iter().enumerate() {
            info!("Processing DEX transaction {}/{}", i + 1, signatures.len());
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

        info!("Completed DEX trade analysis. Found {} hot volumes", hot_volumes.len());
        Ok(hot_volumes)
    }

    async fn track_amm_swaps(&self) -> Result<Vec<TradingVolume>, Box<dyn std::error::Error + Send + Sync>> {
        let amm_program_id = Pubkey::from_str(RAYDIUM_AMM_PROGRAM)?;
        info!("Fetching signatures for AMM program {}", RAYDIUM_AMM_PROGRAM);
        let signatures = self.rpc_client.get_signatures_for_address(&amm_program_id)?;
        info!("Found {} AMM transactions to analyze", signatures.len());
        let mut hot_volumes = Vec::new();

        for (i, sig_info) in signatures.iter().enumerate() {
            info!("Processing AMM transaction {}/{}", i + 1, signatures.len());
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

        info!("Completed AMM swap analysis. Found {} hot volumes", hot_volumes.len());
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

            let token_price = match self.get_token_price(&post.mint).await {
                Ok(price) => price,
                Err(e) => {
                    warn!("Failed to get price for token {}: {}", post.mint, e);
                    continue;
                }
            };

            let trade_value = amount_change * token_price;
            info!("Token {} trade value: ${:.2}", post.mint, trade_value);

            if trade_value >= self.min_volume && trade_value <= self.max_volume {
                let token_name = match self.get_token_name(&post.mint).await {
                    Ok(name) => name,
                    Err(e) => {
                        warn!("Failed to get name for token {}: {}", post.mint, e);
                        post.mint.clone()
                    }
                };

                if let Some(existing) = hot_volumes.iter_mut().find(|v| v.token_address == post.mint) {
                    info!("Updating existing volume for {}: Adding ${:.2}", token_name, trade_value);
                    existing.total_volume += trade_value;
                    existing.trade_count += 1;
                    existing.average_trade_size = existing.total_volume /
                        (existing.trade_count as f64 + existing.swap_count as f64);
                    existing.last_update = SystemTime::now();
                } else {
                    info!("Adding new volume for {}: ${:.2}", token_name, trade_value);
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
        info!("Getting hot pairs...");
        let hot_pairs = self.volume_data
            .values()
            .filter(|v| {
                v.average_trade_size >= self.min_volume
                && v.average_trade_size <= self.max_volume
                && (v.trade_count + v.swap_count) >= 3
            })
            .cloned()
            .collect::<Vec<_>>();

        info!("Found {} hot pairs", hot_pairs.len());
        hot_pairs
    }

    async fn get_token_price(&self, mint: &str) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
        info!("Fetching price for token {}", mint);
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
            info!("Price for token {}: ${}", mint, token_data.price);
            Ok(token_data.price)
        } else {
            warn!("Price not found for token {}", mint);
            Err("Price not found on Raydium".into())
        }
    }

    fn clean_old_data(&mut self) {
        info!("Cleaning old data...");
        let now = SystemTime::now();
        let initial_count = self.volume_data.len();
        self.volume_data.retain(|_, v| {
            if let Ok(duration) = now.duration_since(v.last_update) {
                duration < self.time_window
            } else {
                false
            }
        });
        let removed_count = initial_count - self.volume_data.len();
        info!("Cleaned {} old entries", removed_count);
    }

    async fn get_token_name(&self, mint: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(name) = self.token_names_cache.get(mint) {
            info!("Found cached name for token {}: {}", mint, name);
            return Ok(name.clone());
        }

        info!("No cached name found for token {}, using mint address", mint);
        Ok(mint.to_string())
    }
}