use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Solana RPC error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),

    #[error("RPC error: {0}")]
    RpcStringError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),

    #[error("Pool error: {0}")]
    PoolError(String),

    #[error("Build error: {0}")]
    BuildError(#[from] deadpool_postgres::BuildError),

    #[error("TLS error: {0}")]
    TlsError(#[from] native_tls::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Shutdown error: {0}")]
    ShutdownError(String),

    #[error("Unexpected error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, IndexerError>;
