mod execution;
mod whale;
mod solana_config;
mod dex;
pub mod strategy;

use tokio;
use log::{info, error};
use dotenv::dotenv;
use std::env;

// Import necessary components
// use crate::solana_whale_trader::{
//     whale::WhaleDetector,
//     whale::config::WhaleConfig,
//     execution::TradeExecutor,
//     strategy::StrategyConfig,
// };

use crate::solana_config::SolanaConfig;
use crate::whale::WhaleDetector;
use crate:: whale::config::WhaleConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();

    // Initialize logging
    env_logger::init();

    let config = WhaleConfig::new();
    let whale_detector = WhaleDetector::new(
        config
    );

    // Start the trading bot
    info!("Starting Solana Whale Tracking Trading Bot");
    whale_detector.start().await?;

    Ok(())
}