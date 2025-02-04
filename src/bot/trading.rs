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
use std::time::{SystemTime, Duration};
use solana_program::pubkey::Pubkey;
use std::str::FromStr;
use log::{info, warn, error};
use std::collections::{HashMap, HashSet};

const RAYDIUM_DEX_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const RAYDIUM_AMM_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

#[derive(Clone, Debug)]
pub struct TokenInfo {
    pub symbol: String,
    pub address: String,
}

impl std::fmt::Display for TokenInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

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

#[derive(Clone, Debug)]
pub struct VolumeThreshold {
    pub min_volume: f64,
    pub max_volume: f64,
    pub timeframe: Duration,
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
    time_window: Duration,
}

#[allow(dead_code)]
pub struct VolumeTracker {
    rpc_client: Arc<RpcClient>,
    volume_thresholds: HashMap<String, VolumeThreshold>,
    volume_data: HashMap<String, TradingVolume>,
    token_names_cache: HashMap<String, String>,
    price_cache: HashMap<String, (f64, SystemTime)>,
    pub monitored_tokens: HashSet<String>,
}

impl VolumeTracker {
    pub fn new(rpc_url: &str, min_volume: f64, max_volume: f64) -> Self {
        info!("Initializing VolumeTracker with min_volume: ${}, max_volume: ${}", min_volume, max_volume);
        Self {
            rpc_client: Arc::new(RpcClient::new(rpc_url.to_string())),
            volume_thresholds: HashMap::new(),
            volume_data: HashMap::new(),
            token_names_cache: HashMap::new(),
            price_cache: HashMap::new(),
            monitored_tokens: HashSet::new(),
        }
    }

    pub fn get_monitored_tokens_list(&self) -> String {
        self.monitored_tokens
            .iter()
            .map(|addr| self.token_names_cache.get(addr).unwrap_or(addr))
            .collect::<Vec<_>>()
            .as_slice()  // Convert the Vec to a slice
            .join(", ")  // Now you can call join on the slice
    }

    pub fn set_token_volume_threshold(&mut self, token_address: String, min_volume: f64, max_volume: f64, timeframe_minutes: u64) {
        let threshold = VolumeThreshold {
            min_volume,
            max_volume,
            timeframe: Duration::from_secs(timeframe_minutes * 60),
        };
        self.volume_thresholds.insert(token_address.clone(), threshold);
        info!("Set threshold for token {}: min=${}, max=${}, timeframe={}min",
            token_address, min_volume, max_volume, timeframe_minutes);
    }

    // Updated add_monitored_token to include default thresholds
    pub async fn add_monitored_token(&mut self, token_symbol: &str) -> Result<TokenInfo, Box<dyn std::error::Error + Send + Sync>> {
        let token_info = self.get_token_info(token_symbol).await?;
        self.monitored_tokens.insert(token_info.address.clone());

        // Set default thresholds if none exist
        if !self.volume_thresholds.contains_key(&token_info.address) {
            self.set_token_volume_threshold(
                token_info.address.clone(),
                5000.0,  // default min volume
                10000.0, // default max volume
                60      // default timeframe (1 hour)
            );
        }

        Ok(token_info)
    }

    pub fn remove_monitored_token(&mut self, token_address: &str) {
        self.monitored_tokens.remove(token_address);
        self.volume_thresholds.remove(token_address);
        info!("Removed monitoring for token: {}", token_address);
    }

        // New method to get token info from Raydium
        pub async fn get_token_info(&self, token_symbol: &str) -> Result<TokenInfo, Box<dyn std::error::Error + Send + Sync>> {
            let url = "https://api.raydium.io/v2/main/tokens";
            let response = reqwest::Client::new()
                .get(url)
                .send()
                .await?
                .json::<serde_json::Value>()
                .await?;

            if let Some(tokens) = response.as_object() {
                for (address, info) in tokens {
                    if let Some(symbol) = info.get("symbol").and_then(|s| s.as_str()) {
                        if symbol.to_uppercase() == token_symbol.to_uppercase() {
                            return Ok(TokenInfo {
                                symbol: symbol.to_string(),
                                address: address.clone(),
                            });
                        }
                    }
                }
            }
            Err("Token not found on Raydium".into())
        }

        pub async fn track_trades(&mut self) -> Result<Vec<TradingVolume>, Box<dyn std::error::Error + Send + Sync>> {
            info!("Starting trade tracking cycle for {} monitored tokens", self.monitored_tokens.len());
            if self.monitored_tokens.is_empty() {
                info!("No tokens being monitored. Please add tokens using /monitorToken");
                return Ok(Vec::new());
            }

            let mut new_volumes = Vec::new();

            // Track DEX trades for monitored tokens
            let dex_volumes = self.track_dex_trades().await?;

            // Track AMM swaps for monitored tokens
            let swap_volumes = self.track_amm_swaps().await?;

            // Update volume data for each token
            for volume in dex_volumes.into_iter().chain(swap_volumes) {
                if let Some(existing) = self.volume_data.get_mut(&volume.token_address) {
                    existing.total_volume += volume.total_volume;
                    existing.trade_count += volume.trade_count;
                    existing.swap_count += volume.swap_count;
                    existing.average_trade_size = existing.total_volume /
                        (existing.trade_count + existing.swap_count) as f64;
                    existing.last_update = SystemTime::now();
                    new_volumes.push(existing.clone());
                } else {
                    self.volume_data.insert(volume.token_address.clone(), volume.clone());
                    new_volumes.push(volume);
                }
            }

            self.clean_old_data();
            info!("Completed trade tracking cycle. Found {} volumes for monitored tokens", new_volumes.len());
            Ok(new_volumes)
        }

        async fn track_dex_trades(&self) -> Result<Vec<TradingVolume>, Box<dyn std::error::Error + Send + Sync>> {
            let dex_program_id = Pubkey::from_str(RAYDIUM_DEX_PROGRAM)?;
            info!("Fetching signatures for DEX program");

            // Get transactions
            let signatures = self.rpc_client.get_signatures_for_address(&dex_program_id)?;
            info!("Analyzing {} DEX transactions for {} monitored tokens",
                signatures.len(), self.monitored_tokens.len());

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

                // Process transaction if it contains token balances
                if let Some(meta) = tx.transaction.meta {
                    if let Some(pre_balances) = <OptionSerializer<Vec<UiTransactionTokenBalance>> as Into<Option<Vec<UiTransactionTokenBalance>>>>::into(meta.pre_token_balances) {
                        // Check if any monitored tokens are involved in this transaction
                        let has_monitored_token = pre_balances.iter()
                            .any(|balance| self.monitored_tokens.contains(&balance.mint));

                        if has_monitored_token {
                            self.process_token_balances(
                                &pre_balances,
                                meta.post_token_balances.unwrap(),
                                &mut hot_volumes
                            ).await?;
                        }
                    }
                }
            }

            info!("Found {} DEX trades for monitored tokens", hot_volumes.len());
            Ok(hot_volumes)
        }

        async fn track_amm_swaps(&self) -> Result<Vec<TradingVolume>, Box<dyn std::error::Error + Send + Sync>> {
            let amm_program_id = Pubkey::from_str(RAYDIUM_AMM_PROGRAM)?;
            info!("Fetching signatures for AMM program");

            let signatures = self.rpc_client.get_signatures_for_address(&amm_program_id)?;
            info!("Analyzing {} AMM transactions for {} monitored tokens",
                signatures.len(), self.monitored_tokens.len());

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
                    if let Some(pre_balances) = <OptionSerializer<Vec<UiTransactionTokenBalance>> as Into<Option<Vec<UiTransactionTokenBalance>>>>::into(meta.pre_token_balances) {
                        // Check for monitored tokens
                        let has_monitored_token = pre_balances.iter()
                            .any(|balance| self.monitored_tokens.contains(&balance.mint));

                        if has_monitored_token {
                            self.process_token_balances(
                                &pre_balances,
                                meta.post_token_balances.unwrap(),
                                &mut hot_volumes
                            ).await?;
                        }
                    }
                }
            }

            info!("Found {} AMM swaps for monitored tokens", hot_volumes.len());
            Ok(hot_volumes)
        }

    async fn process_token_balances(
        &self,
        pre_balances: &[UiTransactionTokenBalance],
        post_balances: Vec<UiTransactionTokenBalance>,
        hot_volumes: &mut Vec<TradingVolume>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for (pre, post) in pre_balances.iter().zip(post_balances) {
            // Skip if not monitoring this token
            if !self.monitored_tokens.contains(&post.mint) {
                continue;
            }

            // Get threshold for this token
            let threshold = if let Some(t) = self.volume_thresholds.get(&post.mint) {
                t
            } else {
                info!("No threshold set for monitored token: {}", post.mint);
                continue;
            };

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

            if trade_value >= threshold.min_volume && trade_value <= threshold.max_volume {
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
                        time_window: Duration::from_secs(60 * 60)
                    });
                }
            }
        }
        Ok(())
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

        // Use a default timeframe of 1 hour if no specific threshold is set
        let default_timeframe = Duration::from_secs(60 * 60);

        self.volume_data.retain(|token_address, v| {
            // Try to get the threshold for this token, otherwise use default
            let timeframe = self.volume_thresholds.get(token_address)
                .map(|threshold| threshold.timeframe)
                .unwrap_or(default_timeframe);

            if let Ok(duration) = now.duration_since(v.last_update) {
                duration < timeframe
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