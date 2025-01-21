use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Keypair,
    pubkey::Pubkey,
};
use std::str::FromStr;

#[derive(Clone)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub commitment: CommitmentConfig,
    pub keypair: Keypair,
}

impl SolanaConfig {
    pub fn mainnet_default(keypair: Keypair) -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            commitment: CommitmentConfig::confirmed(),
            keypair,
        }
    }

    pub fn create_rpc_client(&self) -> RpcClient {
        RpcClient::new_with_commitment(
            self.rpc_url.clone(),
            self.commitment.clone()
        )
    }

    pub fn websocket_url(&self) -> String {
        self.rpc_url
            .replace("https://", "wss://")
            .replace("http://", "wss://")
    }
}