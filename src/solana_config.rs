use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Keypair,
};

pub struct SolanaConfig {
    pub rpc_url: String,
    pub commitment: CommitmentConfig,
    pub keypair: Keypair,
}

impl SolanaConfig {
    // Default mainnet configuration
    pub fn mainnet_default(keypair: Keypair) -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            commitment: CommitmentConfig::confirmed(),
            keypair,
        }
    }

    // Alternative constructor for different networks
    pub fn custom(
        rpc_url: String,
        commitment: CommitmentConfig,
        keypair: Keypair
    ) -> Self {
        Self {
            rpc_url,
            commitment,
            keypair,
        }
    }

    // Create RPC client
    pub fn create_rpc_client(&self) -> RpcClient {
        RpcClient::new_with_commitment(
            self.rpc_url.clone(),
            self.commitment.clone()
        )
    }

    // Websocket URL derivation
    pub fn websocket_url(&self) -> String {
        // Convert RPC URL to websocket URL
        self.rpc_url
            .replace("https://", "wss://")
            .replace("http://", "wss://")
    }
}

// Usage example
fn main() {
    let keypair = load_keypair(); // Your keypair loading method
    let solana_config = SolanaConfig::mainnet_default(keypair);

    let rpc_client = solana_config.create_rpc_client();
    let websocket_url = solana_config.websocket_url();
}