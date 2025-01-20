mod execution;
mod strategy;
mod whale;
mod solana_config;

// Optional: re-export key types if needed
mod dex;

pub use dex::{
    DexAnalyzer,
    types::{
        DexTransaction,
        TradeType,
        DexProtocol,
        DexTrade,
    },
};
pub use execution::TradeExecutor;
pub use strategy::{
    StrategyAnalyzer,
    types::{
        StrategyConfig,
        RiskParams,
        TradeSignal,
    },
};
pub use whale::WhaleDetector;
pub use solana_config::SolanaConfig;