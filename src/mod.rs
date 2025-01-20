mod dex;
mod execution;
mod strategy;
mod whale;

// Optional: re-export key types if needed
pub use dex::DexAnalyzer;
pub use execution::TradeExecutor;
pub use strategy::StrategyAnalyzer;
pub use whale::WhaleDetector;