use std::sync::Arc;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use tracing::info;
use crate::error::Result;
use crate::config::Config;

pub mod parser;

pub struct Indexer {
    rpc_client: Arc<RpcClient>,
    config: Config,
}

impl Indexer {
    pub fn new(config: Config) -> Self {
        let commitment_config = match config.commitment.to_lowercase().as_str() {
            "finalized" => CommitmentConfig::finalized(),
            "confirmed" => CommitmentConfig::confirmed(),
            "processed" => CommitmentConfig::processed(),
            _ => CommitmentConfig::finalized(),
        };
        
        // We use the URL from the config to create the RpcClient
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            commitment_config,
        ));

        Self {
            rpc_client,
            config,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut current_slot = self.config.start_slot.unwrap_or_else(|| {
            // If no start slot is provided, we start from the current slot
            self.rpc_client.get_slot().unwrap_or(0)
        });

        info!("Starting indexer from slot: {}", current_slot);

        loop {
            // Fetch the block at the current slot
            match self.rpc_client.get_block(current_slot) {
                Ok(block) => {
                    info!("Processing block at slot: {}", current_slot);
                    
                    // Iterate through transactions in the block
                    for tx_with_meta in block.transactions {
                        if let Some(parsed_tx) = parser::parse_transaction(tx_with_meta, current_slot, block.block_time) {
                            info!("Parsed transaction: {}", parsed_tx.signature);
                        }
                    }
                    current_slot += 1;
                }
                Err(_e) => {
                    // Simple error handling: if block is not available yet, wait.
                    // In a production app, we'd handle different RPC error codes more gracefully.
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    
                    // Optional: fetch latest slot to ensure we haven't fallen too far behind
                    if let Ok(latest_slot) = self.rpc_client.get_slot() {
                        if current_slot > latest_slot {
                             tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                }
            }
        }
    }
}
