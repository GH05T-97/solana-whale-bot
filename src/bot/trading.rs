#![allow(dead_code)]
#![allow(unused_variables)]
use std::sync::Arc;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_client::rpc_config::RpcSignaturesForAddressConfig;
use solana_sdk::{commitment_config::CommitmentConfig, account::Account};
use solana_rpc_client_api::config::GetConfirmedSignaturesForAddress2Config;
use solana_transaction_status::{
    option_serializer::OptionSerializer,
    UiTransactionEncoding,
    UiTransactionTokenBalance,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
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

#[derive(Debug, Deserialize)]
struct TokenPrice {
    price: f64,
    #[serde(rename = "mint")]
    token_mint: String,
}

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

#[derive(Clone)]
pub struct TradingVolume {
    pub token_address: String,
    pub token_name: String,
    pub total_volume: f64,
    pub trade_count: u32,
    pub swap_count: u32,
    pub average_trade_size: f64,
    pub last_update: SystemTime,
}

pub struct VolumeTracker {
    rpc_client: Arc<RpcClient>,
    pub min_volume: f64,
    pub max_volume: f64,
    volume_data: HashMap<String, TradingVolume>,
    time_window: Duration,
    token_names_cache: HashMap<String, String>,
    price_cache: HashMap<String, (f64, SystemTime)>,
    pub monitored_tokens: HashSet<String>,
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
            monitored_tokens: HashSet::new(),
        }
    }

    pub async fn add_monitored_token(&mut self, token_symbol: &str) -> Result<TokenInfo, Box<dyn std::error::Error + Send + Sync>> {
        let token_info = self.get_token_info(token_symbol).await?;
        self.monitored_tokens.insert(token_info.address.clone());
        info!("Added token {} ({}) to monitoring", token_info.symbol, token_info.address);
        Ok(token_info)
    }

    pub fn set_token_volume_threshold(&mut self, token_address: String, min: f64, max: f64, timeframe: u64) {
        self.min_volume = min;
        self.max_volume = max;
        self.time_window = Duration::from_secs(timeframe * 60);
        info!("Updated volume thresholds for token {}: min=${}, max=${}, timeframe={}min",
            token_address, min, max, timeframe);
    }

    pub fn remove_monitored_token(&mut self, token_address: &str) {
        if self.monitored_tokens.remove(token_address) {
            info!("Removed token {} from monitoring", token_address);
        }
    }

    pub fn get_monitored_tokens_list(&self) -> String {
        let tokens: Vec<_> = self.monitored_tokens
            .iter()
            .filter_map(|addr| {
                self.token_names_cache.get(addr).map(|name| format!("{} ({})", name, addr))
            })
            .collect();

        if tokens.is_empty() {
            "No tokens monitored".to_string()
        } else {
            tokens.join(", ")
        }
    }

    pub async fn track_trades(&mut self) -> Result<Vec<TradingVolume>, Box<dyn std::error::Error + Send + Sync>> {
        if self.monitored_tokens.is_empty() {
            info!("No tokens being monitored");
            return Ok(Vec::new());
        }

        info!("Starting trade tracking cycle");
        let mut all_volumes = Vec::new();

        // Track DEX trades
        info!("Tracking DEX trades...");
        let dex_volumes = self.track_dex_trades().await?;
        info!("Found {} DEX trading volumes", dex_volumes.len());
        all_volumes.extend(dex_volumes);

        // Track AMM swaps
        info!("Tracking AMM swaps...");
        let swap_volumes = self.track_amm_swaps().await?;
        info!("Found {} AMM swap volumes", swap_volumes.len());

        // Merge volumes
        for swap_vol in swap_volumes {
            if let Some(existing) = all_volumes.iter_mut().find(|v| v.token_address == swap_vol.token_address) {
                info!("Merging volumes for {}: Adding ${:.2} from swaps",
                    swap_vol.token_name, swap_vol.total_volume);
                existing.total_volume += swap_vol.total_volume;
                existing.swap_count += swap_vol.swap_count;
                existing.average_trade_size = existing.total_volume /
                    (existing.trade_count as f64 + existing.swap_count as f64);
            } else {
                info!("Adding new swap volume for {}: ${:.2}",
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
        let mut all_signatures = Vec::new();
        let mut before = None;

        loop {
            let batch = self.rpc_client.get_signatures_for_address_with_config(
                &dex_program_id,
                GetConfirmedSignaturesForAddress2Config {
                    before: before.map(|s| s.signature.clone()),
                    until: None,
                    limit: Some(100),
                    commitment: Some(CommitmentConfig::confirmed()),
                    min_context_slot: None,
                }
            )?;

            if batch.is_empty() {
                break;
            }

            info!("Fetched batch of {} transactions", batch.len());
            before = Some(batch.last().unwrap().signature.clone());
            all_signatures.extend(batch);

            if all_signatures.len() >= 1000 {
                break;
            }
        }

        info!("Found {} DEX transactions to analyze", all_signatures.len());
        let mut hot_volumes = Vec::new();

        for (i, sig_info) in all_signatures.iter().enumerate() {
            if i % 50 == 0 {
                info!("Processing batch {}-{}", i, i+50);
            }

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
            if trade_value >= self.min_volume && trade_value <= self.max_volume {
                let token_name = match self.get_token_name(&post.mint).await {
                    Ok(name) => name,
                    Err(e) => {
                        warn!("Failed to get name for token {}: {}", post.mint, e);
                        post.mint.clone()
                    }
                };

                if let Some(existing) = hot_volumes.iter_mut().find(|v| v.token_address == post.mint) {
                    existing.total_volume += trade_value;
                    existing.trade_count += 1;
                    existing.average_trade_size = existing.total_volume /
                        (existing.trade_count as f64 + existing.swap_count as f64);
                    existing.last_update = SystemTime::now();
                    info!("Updated volume for {}: ${:.2}", token_name, trade_value);
                } else {
                    let token_name_clone = token_name.clone();  // Clone here
                    hot_volumes.push(TradingVolume {
                        token_address: post.mint.clone(),
                        token_name,
                        total_volume: trade_value,
                        trade_count: 1,
                        swap_count: 0,
                        average_trade_size: trade_value,
                        last_update: SystemTime::now(),
                    });
                    info!("New trade tracked for {}: ${:.2}", token_name_clone, trade_value);
                }
            }
        }
        Ok(())
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

    async fn get_token_name(&self, mint: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(name) = self.token_names_cache.get(mint) {
            return Ok(name.clone());
        }
        Ok(mint.to_string())
    }

    pub async fn get_token_info(&self, token_symbol: &str) -> Result<TokenInfo, Box<dyn std::error::Error + Send + Sync>> {
        let url = "https://api-v3.raydium.io/mint/list";
        let client = reqwest::Client::new();

        let response = client.get(url).send().await?;
        info!("API Status: {}", response.status());

        let text = response.text().await?;
        info!("Raw response: {}", text);

        // Parse the JSON after logging
        let json: serde_json::Value = serde_json::from_str(&text)?;

        if let Some(tokens) = json.get("data").and_then(|d| d.get("mintList").and_then(|m| m.as_array())) {
            for token in tokens {
                if let (Some(symbol), Some(address)) = (
                    token.get("symbol").and_then(|s| s.as_str()),
                    token.get("address").and_then(|a| a.as_str())
                ) {
                    if symbol.to_uppercase() == token_symbol.to_uppercase() {
                        return Ok(TokenInfo {
                            symbol: symbol.to_string(),
                            address: address.to_string(),
                        });
                    }
                }
            }
        }
        Err(format!("Token {} not found on Raydium", token_symbol).into())
    }

    fn clean_old_data(&mut self) {
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
        if removed_count > 0 {
            info!("Cleaned {} old entries", removed_count);
        }
    }

    pub fn get_hot_pairs(&self) -> Vec<TradingVolume> {
        let hot_pairs: Vec<_> = self.volume_data
            .values()
            .filter(|v|
                self.monitored_tokens.contains(&v.token_address) &&
                v.average_trade_size >= self.min_volume &&
                v.average_trade_size <= self.max_volume &&
                (v.trade_count + v.swap_count) >= 3
            )
            .cloned()
            .collect();

        info!("Found {} hot pairs", hot_pairs.len());
        hot_pairs
    }
}