pub mod protocols;
pub mod analyzer;
pub mod types;  // Change from 'mod types;' to 'pub mod types;'

pub use protocols::{
    jupiter::JupiterProtocol,
    raydium::RaydiumProtocol,
};

pub use analyzer::DexAnalyzer;
pub use types::{
    DexTransaction,
    TradeType,
    DexProtocol,
    DexTrade,
};