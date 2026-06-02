use serde::Deserialize;
use std::env;
use crate::error::Result;
use tracing::{info, warn, error};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub database_url: String,
    pub start_slot: Option<u64>,
    pub commitment: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        info!("Loading configuration from environment variables...");

        let rpc_url = env::var("RPC_URL")
            .map_err(|_| {
                error!("Critical configuration error: RPC_URL environment variable is not set. Please set RPC_URL to a valid Solana RPC endpoint.");
                crate::error::IndexerError::ConfigError("RPC_URL must be set".to_string())
            })?;
        
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| {
                error!("Critical configuration error: DATABASE_URL environment variable is not set. Please set DATABASE_URL to a valid PostgreSQL connection string.");
                crate::error::IndexerError::ConfigError("DATABASE_URL must be set".to_string())
            })?;

        let start_slot = env::var("START_SLOT")
            .ok()
            .and_then(|s| {
                s.parse().map_err(|e| {
                    warn!("Configuration warning: Invalid START_SLOT value '{}': {}. Will determine starting slot automatically.", s, e);
                    e
                }).ok()
            });

        let commitment = env::var("COMMITMENT")
            .unwrap_or_else(|_| {
                info!("COMMITMENT not specified, using default value 'finalized'");
                "finalized".to_string()
            });

        // Validate commitment level
        if !["processed", "confirmed", "finalized"].contains(&commitment.as_str()) {
            warn!("Configuration warning: Invalid COMMITMENT value '{}'. Valid values are 'processed', 'confirmed', 'finalized'. Using 'finalized' as fallback.", commitment);
        }

        info!("Configuration loaded successfully - RPC: {}, Commitment: {}, Start slot: {:?}", 
              rpc_url, commitment, start_slot);

        Ok(Config {
            rpc_url,
            database_url,
            start_slot,
            commitment,
        })
    }
}
