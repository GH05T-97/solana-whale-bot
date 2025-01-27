pub mod protocols {
    pub mod jupiter;
    pub mod raydium;
}
pub mod analyzer;
pub mod types;

use protocols::{
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