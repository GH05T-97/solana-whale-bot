use solana_client::rpc_client::RpcClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use solana_sdk::signature::Signature;

#[derive(Clone)]
pub struct TradingVolume {
    token_address: String,
    pub token_name: String,
    pub total_volume: f64,
    pub trade_count: u32,
    pub average_trade_size: f64,
    last_update: SystemTime,
}

#[derive(Clone)]
pub struct VolumeTracker {
    rpc_client: Arc<RpcClient>,
    pub min_volume: f64,
    pub max_volume: f64,
    volume_data: HashMap<String, TradingVolume>,
    time_window: Duration,
    token_names_cache: HashMap<String, String>, // Add this
    price_cache: HashMap<String, (f64, SystemTime)>, // Add this for price caching
}


#[derive(Deserialize)]
struct RaydiumPriceResponse {
    data: HashMap<String, TokenPrice>,
}

#[derive(Deserialize)]
struct TokenPrice {
    price: f64,
    #[serde(rename = "mint")]
    token_mint: String,
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

    pub async fn track_trades(&mut self) -> Result<Vec<TradingVolume>, Box<dyn std::error::Error>> {
		let dex_program_id = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".parse()?;

		let signatures = self.rpc_client.get_signatures_for_address(&dex_program_id)?;

		let mut hot_volumes: Vec<TradingVolume> = Vec::new();

		for sig_info in signatures {
			let signature = sig_info.signature.parse::<Signature>()?;

			let tx = self.rpc_client.get_transaction_with_config(
				&signature,
				RpcTransactionConfig {
					encoding: Some(UiTransactionEncoding::Json),
					commitment: Some(CommitmentConfig::confirmed()),
					max_supported_transaction_version: Some(0),
				},
			)?;

			if let Some(meta) = tx.transaction.meta {
				if let Some(token_balances) = meta.pre_token_balances.into_option() {
					for (pre, post) in token_balances.iter().zip(meta.post_token_balances.unwrap()) {
						let amount_change = (post.ui_token_amount.ui_amount.unwrap_or(0.0)
							- pre.ui_token_amount.ui_amount.unwrap_or(0.0)).abs();

						// Get token price (you'd need to implement get_token_price)
						let token_price = self.get_token_price(&post.mint).await?;
						let trade_value = amount_change * token_price;

						// Check if trade is within our target range
						if trade_value >= self.min_volume && trade_value <= self.max_volume {
							let token_name = self.get_token_name(&post.mint).await?;

							// Update or create trading volume entry
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

		// Clean up old data
		self.clean_old_data();

		Ok(hot_volumes)
    }

    pub fn get_hot_pairs(&self) -> Vec<&TradingVolume> {
        self.volume_data
            .values()
            .filter(|v| {
                v.average_trade_size >= self.min_volume
                && v.average_trade_size <= self.max_volume
                && v.trade_count >= 3 // Minimum trades to be considered "hot"
            })
            .collect()
    }

	async fn get_token_price(&self, mint: &str) -> Result<f64, Box<dyn std::error::Error>> {
		let url = format!(
			"https://api.raydium.io/v2/main/price?tokens={}",
			mint
		);

		let client = reqwest::Client::new();
		let response = client.get(&url)
			.send()
			.await?
			.json::<RaydiumPriceResponse>()
			.await;

		match response {
			Ok(data) => {
				if let Some(token_data) = data.data.get(mint) {
					Ok(token_data.price)
				} else {
					// Fallback to another source or return error
					Err("Price not found on Raydium".into())
				}
			}
			Err(_) => {
				// Handle API error or fallback
				Err("Failed to fetch price from Raydium".into())
			}
		}
	}

    async fn get_token_name(&self, mint: &str) -> Result<String, Box<dyn std::error::Error>> {
        // First check our cache
        if let Some(name) = self.token_names_cache.get(mint) {
            return Ok(name.clone());
        }

        // If not in cache, fetch from Solana
        let token_account = self.rpc_client.get_account(&mint.parse()?)?;

        // Parse metadata from token account
        if let Ok(metadata) = spl_token_metadata::state::Metadata::from_account_info(&token_account) {
            let name = metadata.data.name.trim_matches(char::from(0)).to_string();

            // Cache the result
            self.token_names_cache.insert(mint.to_string(), name.clone());

            Ok(name)
        } else {
            // If metadata isn't available, return mint address as fallback
            Ok(mint.to_string())
        }
    }
}