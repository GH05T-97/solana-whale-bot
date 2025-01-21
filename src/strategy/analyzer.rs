use std::{
    collections::HashMap,
    sync::Arc,
};
use super::types::{StrategyConfig, RiskParams};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
// Common imports to add
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::{HashMap, HashSet};
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};
use crate::{
    whale::types::WhaleMovement,
    strategy::types::{TradeSignal, TradeDirection}, // Modify this line
};

use rust_decimal::prelude::FromPrimitive;

pub struct RiskManager {
    risk_params: RiskParams,
}

impl RiskManager {
    pub fn new(risk_params: RiskParams) -> Self {
        Self { risk_params }
    }
}

pub struct StrategyAnalyzer {
    config: StrategyConfig,
    risk_manager: Arc<RiskManager>,
    portfolio_state: Arc<RwLock<PortfolioState>>,
    active_trades: Arc<RwLock<HashMap<Pubkey, TradeInfo>>>,
}

impl StrategyAnalyzer {

    pub fn new(config: StrategyConfig) -> Self {
        Self {
            config,
            risk_manager: Arc::new(RiskManager::new(config.risk_params)),
            portfolio_state: Arc::new(RwLock::new(PortfolioState::default())),
            active_trades: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn analyze_whale_movement(&self, movement: &WhaleMovement) -> Option<TradeSignal> {
        // Calculate position size first
        let position_size = self.calculate_position_size(movement).await?;

        // Create initial trade signal
        let signal = self.create_trade_signal(movement, position_size).await?;

        // Validate the trade
        if !self.validate_trade(&signal).await {
            return None;
        }

        Some(signal)
    }

    async fn create_trade_signal(&self, movement: &WhaleMovement, size: Decimal) -> Option<TradeSignal> {
        let direction = match &movement.movement_type {
            MovementType::TokenSwap { action, .. } => match action.as_str() {
                "buy" => TradeDirection::Long,
                "sell" => TradeDirection::Short,
                _ => return None,
            },
            _ => return None,
        };

        // Calculate price levels
        let entry_price = Decimal::from_f64(movement.price).unwrap_or_else(|| Decimal::new(0, 0));;
        let stop_loss_pct = Decimal::new(2, 2).unwrap_or_else(|| Decimal::new(0, 0));; // 2% stop loss
        let take_profit_pct = Decimal::new(6, 2).unwrap_or_else(|| Decimal::new(0, 0));; // 6% take profit

        let (stop_loss, take_profit) = match direction {
            TradeDirection::Long => (
                entry_price * (Decimal::ONE - stop_loss_pct),
                entry_price * (Decimal::ONE + take_profit_pct)
            ),
            TradeDirection::Short => (
                entry_price * (Decimal::ONE + stop_loss_pct),
                entry_price * (Decimal::ONE - take_profit_pct)
            ),
            TradeDirection::None => return None,
        };

        Some(TradeSignal {
            direction,
            token: movement.token_address.clone(),
            size,
            entry_price,
            stop_loss: Some(stop_loss),
            take_profit: Some(take_profit),
            confidence: movement.confidence,
            source_whale: movement.whale_address.clone(),
            price_impact: movement.price_impact,
            estimated_slippage: movement.slippage,
        })
    }


    async fn calculate_position_size(&self, movement: &WhaleMovement) -> Option<Decimal> {
        let portfolio = self.portfolio_state.read().await;
        let active_trades = self.active_trades.read().await;

        // Check if we can take new trades
        if active_trades.len() >= self.config.risk_params.max_concurrent_trades as usize {
            return None;
        }

        // Calculate total risk currently in use
        let current_risk: Decimal = active_trades.values()
            .map(|trade| trade.risk_amount)
            .sum();

        // Check if we have risk budget available
        let remaining_risk = self.config.risk_params.max_total_risk - current_risk;
        if remaining_risk <= Decimal::ZERO {
            return None;
        }

        // Calculate position size based on whale's trade, but cap it
        let whale_position_ratio = Decimal::from_f64(0.1)?; // Take 10% of whale's position size
        let whale_size = Decimal::from_u64(movement.amount)?;
        let desired_size = whale_size * whale_position_ratio;

        // Apply our maximum position size limit
        let max_allowed = self.config.risk_params.max_position_size;
        let position_size = desired_size.min(max_allowed);

        // Final check: ensure position risk doesn't exceed per-trade limit
        let risk_amount = self.calculate_position_risk(position_size, movement)?;
        if risk_amount > self.config.risk_params.max_loss_per_trade {
            // Scale down position size to meet risk limit
            let scale_factor = self.config.risk_params.max_loss_per_trade / risk_amount;
            return Some(position_size * scale_factor);
        }

        Some(position_size)
    }

    async fn validate_trade(&self, signal: &TradeSignal) -> bool {
        // Must pass ALL these checks to take the trade
        let portfolio = self.portfolio_state.read().await;
        let active_trades = self.active_trades.read().await;

        // 1. Check confidence threshold
        if signal.confidence < self.config.risk_params.min_confidence {
            return false;
        }

        // 2. Check portfolio value enough for trade
        if portfolio.total_value < self.config.total_portfolio_sol {
            return false;
        }

        // 3. Check concurrent trades limit
        if active_trades.len() >= self.config.risk_params.max_concurrent_trades as usize {
            return false;
        }

        // 4. Check minimum trade size (to account for fees)
        let min_trade_size = Decimal::new(10000000, 9); // 0.01 SOL
        if signal.size < min_trade_size {
            return false;
        }

        // 5. Check price impact and slippage
        if signal.price_impact > self.config.max_price_impact
            || signal.estimated_slippage > self.config.max_slippage {
            return false;
        }

        true
    }

    fn calculate_position_risk(&self, size: Decimal, movement: &WhaleMovement) -> Option<Decimal> {
        // Calculate maximum possible loss based on stop loss
        let entry_price = Decimal::from_f64(movement.price)?;
        let stop_loss_pct = Decimal::new(2, 2); // 2% stop loss

        match movement.movement_type {
            MovementType::TokenSwap { action: "buy", .. } => {
                Some(size * stop_loss_pct)
            },
            MovementType::TokenSwap { action: "sell", .. } => {
                Some(size * stop_loss_pct)
            },
            _ => None,
        }
    }
}