use solana_indexer_rs::{Config, Result};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Solana Indexer...");

    // Load configuration
    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(e);
        }
    };

    info!("Configuration loaded successfully");
    
    // Initialize Indexer
    let indexer = solana_indexer_rs::Indexer::new(config);
    
    info!("Indexer initialization complete. Running...");

    // Start indexer loop
    if let Err(e) = indexer.run().await {
        error!("Indexer error: {}", e);
        return Err(e);
    }

    Ok(())
}
