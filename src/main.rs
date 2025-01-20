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
use crate::solana_whale_trader::{
    whale::WhaleDetector,
    whale::config::WhaleConfig,
    execution::TradeExecutor,
    strategy::StrategyConfig,
};

use crate::solana_config::SolanaConfig;

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();

    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = load_configuration()?;

    // Create keypair (in production, load from secure storage)
    let keypair = load_keypair()?;

    // Create TradeExecutor
    let trade_executor = TradeExecutor::new(
        keypair,
    );

    // Create WhaleDetector with TradeExecutor
    let whale_detector = WhaleDetector::new(
        config.whale_config.clone(),
        trade_executor
    );

    // Start the trading bot
    info!("Starting Solana Whale Tracking Trading Bot");
    whale_detector.start().await?;

    Ok(())
}

fn load_configuration() -> Result<AppConfig, Box<dyn std::error::Error>> {
    // Load configuration from environment or config file
    Ok(AppConfig {
        whale_config: WhaleConfig::load_from_env()?,
        executor_config: ExecutorConfig::load_from_env()?,
        strategy_config: StrategyConfig::load_from_env()?,
    })
}


// Configuration struct to hold different component configurations
struct AppConfig {
    whale_config: WhaleConfig,
    executor_config: ExecutorConfig,
    strategy_config: StrategyConfig,
}