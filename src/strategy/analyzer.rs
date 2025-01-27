use std::{
    collections::HashMap,
    sync::Arc,
};
use super::types::{
    StrategyConfig,
    RiskParams,
    PortfolioState,
    TradeInfo,
    TradeSignal,
    TradeDirection
};
use tokio::sync::RwLock;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use std::collections::HashSet;
use solana_sdk::pubkey::Pubkey;
use chrono::{DateTime, Utc};
use crate::whale::types::{WhaleMovement, MovementType};

#[derive(Debug, Default)]
pub struct RiskManager {
    risk_params: RiskParams,
}

impl RiskManager {
    pub fn new(risk_params: RiskParams) -> Self {
        Self { risk_params }
    }
}

#[derive(Clone, Debug)]
pub struct StrategyAnalyzer {
    config: StrategyConfig,
    risk_manager: Arc<RiskManager>,
    portfolio_state: Arc<RwLock<PortfolioState>>,
    active_trades: Arc<RwLock<HashMap<Pubkey, TradeInfo>>>,
}

impl Default for StrategyAnalyzer {
    fn default() -> Self {
        Self {
            config: StrategyConfig::default(),
            risk_manager: Arc::new(RiskManager::default()),
            portfolio_state: Arc::new(RwLock::new(PortfolioState::default())),
            active_trades: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl StrategyAnalyzer {
    pub fn new(config: StrategyConfig) -> Self {
        Self {
            config: config.clone(),
            risk_manager: Arc::new(RiskManager::new(config.risk_params)),
            portfolio_state: Arc::new(RwLock::new(PortfolioState::default())),
            active_trades: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn analyze_whale_movement(&self, movement: &WhaleMovement) -> Option<TradeSignal> {
        let position_size = self.calculate_position_size(movement).await?;
        let signal = self.create_trade_signal(movement, position_size).await?;

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

        let entry_price = Decimal::from_f64(movement.price)?;
        let stop_loss_pct = Decimal::new(2, 2);
        let take_profit_pct = Decimal::new(6, 2);

        let (stop_loss, take_profit) = match direction {
            TradeDirection::Long => (
                entry_price * (Decimal::ONE - stop_loss_pct),
                entry_price * (Decimal::ONE + take_profit_pct)
            ),
            TradeDirection::Short => (
                entry_price * (Decimal::ONE + stop_loss_pct),
                entry_price * (Decimal::ONE - take_profit_pct)
            ),
        };

        Some(TradeSignal {
            direction,
            token: Pubkey::from_str(&movement.token_address)
                .ok()?,
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

        if active_trades.len() >= self.config.risk_params.max_concurrent_trades as usize {
            return None;
        }

        let current_risk: Decimal = active_trades.values()
            .map(|trade| trade.risk_amount)
            .sum();

        let remaining_risk = self.config.risk_params.max_total_risk - current_risk;
        if remaining_risk <= Decimal::ZERO {
            return None;
        }

        let whale_position_ratio = Decimal::from_f64(0.1)?;
        let whale_size = Decimal::from_f64(movement.amount)?;
        let desired_size = whale_size * whale_position_ratio;

        let max_allowed = self.config.risk_params.max_position_size;
        let position_size = desired_size.min(max_allowed);

        let risk_amount = self.calculate_position_risk(position_size, movement)?;
        if risk_amount > self.config.risk_params.max_loss_per_trade {
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

        match &movement.movement_type {
            MovementType::TokenSwap { action, .. } => match action.as_str() {
                "buy" | "sell" => Some(size * stop_loss_pct),
                _ => None,
            },
            _ => None,
        }
    }
}