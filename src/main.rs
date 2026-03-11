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

    // Initialize Database
    let database = match solana_indexer_rs::Database::new(&config).await {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return Err(e);
        }
    };

    info!("Database initialized and migrations run");
    
    // Initialize Indexer
    let indexer = solana_indexer_rs::Indexer::new(config, database);
    
    info!("Indexer initialization complete. Running...");

    // Start indexer loop
    if let Err(e) = indexer.run().await {
        error!("Indexer error: {}", e);
        return Err(e);
    }

    Ok(())
}
