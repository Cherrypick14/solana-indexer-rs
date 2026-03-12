use std::sync::Arc;
pub mod parser;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use tracing::info;
use crate::error::Result;
use crate::config::Config;

use crate::database::Database;

pub struct Indexer {
    rpc_client: Arc<RpcClient>,
    db: Database,
    config: Config,
}

impl Indexer {
    pub fn new(config: Config, db: Database) -> Self {
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
            db,
            config,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut current_slot = if let Some(slot) = self.config.start_slot {
            slot
        } else {
            match self.db.get_last_indexed_slot().await {
                Ok(slot) if slot > 0 => slot + 1,
                _ => self.rpc_client.get_slot()?
            }
        };

        info!("Starting indexer from slot: {}", current_slot);

        loop {
            // Fetch the block at the current slot
            match self.rpc_client.get_block(current_slot) {
                Ok(block) => {
                    info!("Processing block at slot: {}", current_slot);
                    
                    // Iterate through transactions in the block
                    for tx_with_meta in block.transactions {
                        if let Some(parsed_tx) = parser::parse_transaction(tx_with_meta, current_slot, block.block_time) {
                            if let Err(e) = self.db.save_transaction(&parsed_tx).await {
                                tracing::error!("Failed to save transaction {}: {}", parsed_tx.signature, e);
                            }
                        }
                    }

                    // Update checkpoint
                    if let Err(e) = self.db.update_last_indexed_slot(current_slot).await {
                        tracing::error!("Failed to update checkpoint for slot {}: {}", current_slot, e);
                    }

                    current_slot += 1;
                }
                Err(e) => {
                    // If block is not available yet, wait.
                    // Check if it's a "slot not found" or similar error vs a connection error
                    let error_msg = e.to_string();
                    if error_msg.contains("Slot not found") || error_msg.contains("skipped") {
                        info!("Slot {} not found (possibly skipped), moving to next", current_slot);
                        current_slot += 1;
                        continue;
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    
                    if let O k(latest_slot) = self.rpc_client.get_slot() {
                        if current_slot > latest_slot {
                             tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        }
                    }
                }
            }
        }
    }
}
