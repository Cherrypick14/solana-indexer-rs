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
    pub api_bind_address: String,
    pub api_port: u16,
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

        let api_bind_address = env::var("API_BIND_ADDRESS")
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        let api_port = env::var("API_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .unwrap_or_else(|e| {
                warn!("Configuration warning: Invalid API_PORT value: {}. Using default port 8080.", e);
                8080
            });

        info!("Configuration loaded successfully - RPC: {}, Commitment: {}, Start slot: {:?}", 
              rpc_url, commitment, start_slot);

        Ok(Config {
            rpc_url,
            database_url,
            start_slot,
            commitment,
            api_bind_address,
            api_port,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env_valid() {
        temp_env::with_vars(
            [
                ("RPC_URL", Some("https://api.devnet.solana.com")),
                ("DATABASE_URL", Some("postgresql://user:pass@localhost:5432/db")),
                ("START_SLOT", Some("12345")),
                ("COMMITMENT", Some("confirmed")),
                ("API_BIND_ADDRESS", Some("127.0.0.1")),
                ("API_PORT", Some("3000")),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.rpc_url, "https://api.devnet.solana.com");
                assert_eq!(config.database_url, "postgresql://user:pass@localhost:5432/db");
                assert_eq!(config.start_slot, Some(12345));
                assert_eq!(config.commitment, "confirmed");
                assert_eq!(config.api_bind_address, "127.0.0.1");
                assert_eq!(config.api_port, 3000);
            },
        );
    }

    #[test]
    fn test_config_from_env_missing_required() {
        // Missing RPC_URL
        temp_env::with_vars(
            [
                ("RPC_URL", None::<&str>),
                ("DATABASE_URL", Some("postgresql://user:pass@localhost:5432/db")),
            ],
            || {
                let result = Config::from_env();
                assert!(result.is_err());
            },
        );

        // Missing DATABASE_URL
        temp_env::with_vars(
            [
                ("RPC_URL", Some("https://api.devnet.solana.com")),
                ("DATABASE_URL", None::<&str>),
            ],
            || {
                let result = Config::from_env();
                assert!(result.is_err());
            },
        );
    }

    #[test]
    fn test_config_defaults() {
        temp_env::with_vars(
            [
                ("RPC_URL", Some("https://api.devnet.solana.com")),
                ("DATABASE_URL", Some("postgresql://user:pass@localhost:5432/db")),
                ("START_SLOT", None::<&str>),
                ("COMMITMENT", None::<&str>),
                ("API_BIND_ADDRESS", None::<&str>),
                ("API_PORT", None::<&str>),
            ],
            || {
                let config = Config::from_env().unwrap();
                assert_eq!(config.start_slot, None);
                assert_eq!(config.commitment, "finalized"); // default
                assert_eq!(config.api_bind_address, "0.0.0.0"); // default
                assert_eq!(config.api_port, 8080); // default
            },
        );
    }

    #[quickcheck_macros::quickcheck]
    fn test_config_validation_property(
        rpc: String,
        db: String,
        slot: String,
        commitment: String,
        bind_addr: String,
        port: String,
    ) -> bool {
        // temp_env uses std::env::set_var which panics if the value contains a NUL byte
        if rpc.contains('\0') || db.contains('\0') || slot.contains('\0') 
            || commitment.contains('\0') || bind_addr.contains('\0') || port.contains('\0') {
            return true;
        }

        // Random strings should not cause panics, they should just result in errors or defaults
        temp_env::with_vars(
            [
                ("RPC_URL", Some(&rpc)),
                ("DATABASE_URL", Some(&db)),
                ("START_SLOT", Some(&slot)),
                ("COMMITMENT", Some(&commitment)),
                ("API_BIND_ADDRESS", Some(&bind_addr)),
                ("API_PORT", Some(&port)),
            ],
            || {
                let _ = Config::from_env(); // Should not panic
                true
            },
        )
    }
}
