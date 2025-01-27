// Common imports to add
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use lru::LruCache;
use crate::whale::types::{Transaction, MovementType};

#[derive(Debug)]
pub struct WhaleCache {
    balance_cache: Arc<RwLock<LruCache<String, u64>>>,
    transaction_cache: Arc<RwLock<LruCache<String, Transaction>>>,
    whale_status_cache: Arc<RwLock<HashMap<String, bool>>>, // Cache for whale status
    movement_type_cache: Arc<RwLock<HashMap<String, MovementType>>>, // Cache for movement types
    confidence_cache: Arc<RwLock<HashMap<String, f64>>>, // Cache for confidence levels
}

impl WhaleCache {
    pub fn new() -> Self {
        Self {
            balance_cache: Arc::new(RwLock::new(LruCache::new(1000))), // Cache for balances
            transaction_cache: Arc::new(RwLock::new(LruCache::new(1000))), // Cache for transactions
            whale_status_cache: Arc::new(RwLock::new(HashMap::new())), // Cache for whale status
            movement_type_cache: Arc::new(RwLock::new(HashMap::new())), // Cache for movement types
            confidence_cache: Arc::new(RwLock::new(HashMap::new())), // Cache for confidence levels
        }
    }

    /// Get the cached balance for an address
    pub async fn get_cached_balance(&self, address: &str) -> Option<u64> {
        if let Some(balance) = self.balance_cache.read().await.get(address) {
            return Some(*balance);
        }
        None
    }

    /// Update the cache with a new balance for an address
    pub async fn update_cache(&self, address: String, balance: u64) {
        self.balance_cache.write().await.put(address, balance);
    }

    /// Get the whale status for an address
    pub async fn get_whale_status(&self, address: &str) -> Option<bool> {
        self.whale_status_cache.read().await.get(address).copied()
    }

    /// Set the whale status for an address
    pub async fn set_whale_status(&self, address: &str, is_whale: bool) {
        self.whale_status_cache.write().await.insert(address.to_string(), is_whale);
    }

    /// Get the movement type for a transaction signature
    pub async fn get_movement_type(&self, signature: &str) -> Option<MovementType> {
        self.movement_type_cache.read().await.get(signature).cloned()
    }

    /// Set the movement type for a transaction signature
    pub async fn set_movement_type(&self, signature: String, movement_type: MovementType) {
        self.movement_type_cache.write().await.insert(signature, movement_type);
    }

    /// Get the confidence level for a transaction signature
    pub async fn get_confidence(&self, signature: &str) -> Option<f64> {
        self.confidence_cache.read().await.get(signature).copied()
    }

    /// Set the confidence level for a transaction signature
    pub async fn set_confidence(&self, signature: String, confidence: f64) {
        self.confidence_cache.write().await.insert(signature, confidence);
    }

    /// Update the cache with whale data (address, movement type, and confidence)
    pub async fn update_whale_data(&self, address: &str, movement_type: MovementType, confidence: f64) {
        self.set_movement_type(address.to_string(), movement_type).await;
        self.set_confidence(address.to_string(), confidence).await;
    }

    /// Update the cache with a confirmed whale movement
    pub async fn update_movement_history(&self, movement: WhaleMovement) {
        let signature = movement.transaction.signature.clone();
        self.set_movement_type(signature, movement.movement_type).await;
        self.set_confidence(signature, movement.confidence).await;
    }
}