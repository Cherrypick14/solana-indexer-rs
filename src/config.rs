use serde::Deserialize;
use std::env;
use crate::error::Result;

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

        let rpc_url = env::var("RPC_URL")
            .map_err(|_| crate::error::IndexerError::ConfigError("RPC_URL must be set".to_string()))?;
        
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| crate::error::IndexerError::ConfigError("DATABASE_URL must be set".to_string()))?;

        let start_slot = env::var("START_SLOT")
            .ok()
            .and_then(|s| s.parse().ok());

        let commitment = env::var("COMMITMENT")
            .unwrap_or_else(|_| "finalized".to_string());

        Ok(Config {
            rpc_url,
            database_url,
            start_slot,
            commitment,
        })
    }
}
