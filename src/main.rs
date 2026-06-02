use solana_indexer_rs::{Config, Result, ShutdownManager, GracefulShutdown};
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Solana Indexer...");

    // Set up graceful shutdown handling
    let mut shutdown_manager = ShutdownManager::new().await?;
    let shutdown_rx = shutdown_manager.subscribe();

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

    // Start indexer loop with shutdown handling
    tokio::select! {
        result = indexer.run_with_shutdown(Some(shutdown_rx)) => {
            if let Err(e) = result {
                error!("Indexer error: {}", e);
                return Err(e);
            }
        }
        _ = shutdown_manager.wait_for_shutdown() => {
            info!("Shutdown signal received in main");
        }
    }

    // Perform graceful shutdown
    info!("Performing graceful shutdown...");
    if let Err(e) = indexer.shutdown().await {
        error!("Error during shutdown: {}", e);
        return Err(e);
    }

    info!("Shutdown completed successfully");
    Ok(())
}
