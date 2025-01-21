pub mod execution;
pub mod strategy;
pub mod whale;
pub mod solana_config;

// Optional: re-export key types if needed
pub mod dex;

pub use solana_config::SolanaConfig;


// Re-export key types
pub use dex::{
    DexAnalyzer,
    types::DexProtocol,
    types::DexTransaction,
    types::TradeType,
};

pub use execution::{
    TradeExecutor,
    types::OrderRequest,
    types::OrderResult,
};

pub use strategy::{
    StrategyAnalyzer,
    types::StrategyConfig,
    types::TradeSignal,
    types:: RiskParams
};

pub use whale::{
    WhaleDetector,
    config::WhaleConfig,
};