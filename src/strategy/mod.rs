// Example in strategy/mod.rs
pub mod analyzer;
pub mod types;

pub use analyzer::StrategyAnalyzer;
pub use types::{
    StrategyConfig,
    RiskParams,
    TradeSignal,
};