use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct WhaleConfig {
    pub minimum_balance: u64,        // Minimum balance to be considered a whale
    pub minimum_transaction: u64,    // Minimum transaction size to track
    pub tracked_addresses: HashSet<String>,  // Known whale addresses
}

impl Default for WhaleConfig {
    fn default() -> Self {
        Self {
            minimum_balance: 10_000 * 1_000_000_000,    // 10,000 SOL in lamports
            minimum_transaction: 1_000 * 1_000_000_000,  // 1,000 SOL in lamports
            tracked_addresses: HashSet::new(),
        }
    }
}