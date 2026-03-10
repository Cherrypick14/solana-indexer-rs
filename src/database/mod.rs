use deadpool_postgres::{Config as DeadpoolConfig, Pool, Runtime};
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use crate::error::{Result, IndexerError};
use crate::config::Config;
use crate::models::transaction::TransactionRecord;

pub struct Database {
    pub pool: Pool,
}

impl Database {
    pub async fn new(config: &Config) -> Result<Self> {
        let mut cfg = DeadpoolConfig::new();
        let pg_config = config.database_url.parse::<tokio_postgres::Config>()
            .map_err(|e| IndexerError::ConfigError(format!("Invalid database URL: {}", e)))?;

        cfg.host = pg_config.get_hosts().first().and_then(|h| match h {
            tokio_postgres::config::Host::Tcp(s) => Some(s.clone()),
            _ => None,
        });
        cfg.dbname = pg_config.get_dbname().map(|s| s.to_string());
        cfg.user = pg_config.get_user().map(|s| s.to_string());
        
        if let Some(password) = pg_config.get_password() {
             cfg.password = Some(String::from_utf8_lossy(password).to_string());
        }

        let connector = TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build().map_err(|e| IndexerError::InternalError(e.to_string()))?;
        let tls = MakeTlsConnector::new(connector);

        let pool = cfg.create_pool(Some(Runtime::Tokio1), tls)
            .map_err(|e| IndexerError::InternalError(e.to_string()))?;

        let db = Database { pool };
        db.run_migrations().await?;
        
        Ok(db)
    }

    pub async fn run_migrations(&self) -> Result<()> {
        let client = self.pool.get().await.map_err(|e| IndexerError::InternalError(e.to_string()))?;
        let schema = include_str!("../../migrations/20240309000000_initial_schema.sql");
        client.batch_execute(schema).await.map_err(|e| IndexerError::InternalError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_last_indexed_slot(&self) -> Result<u64> {
        let client = self.pool.get().await.map_err(|e| IndexerError::InternalError(e.to_string()))?;
        let row = client.query_one("SELECT value FROM indexer_state WHERE key = 'last_indexed_slot'", &[])
            .await.map_err(|e| IndexerError::InternalError(e.to_string()))?;
        let value: String = row.get(0);
        Ok(value.parse().unwrap_or(0))
    }

    pub async fn update_last_indexed_slot(&self, slot: u64) -> Result<()> {
        let client = self.pool.get().await.map_err(|e| IndexerError::InternalError(e.to_string()))?;
        client.execute("UPDATE indexer_state SET value = $1 WHERE key = 'last_indexed_slot'", &[&slot.to_string()])
            .await.map_err(|e| IndexerError::InternalError(e.to_string()))?;
        Ok(())
    }

    pub async fn save_transaction(&self, tx: &TransactionRecord) -> Result<()> {
        let mut client = self.pool.get().await.map_err(|e| IndexerError::InternalError(e.to_string()))?;
        let db_tx = client.transaction().await.map_err(|e| IndexerError::InternalError(e.to_string()))?;

        db_tx.execute(
            "INSERT INTO transactions (signature, slot, block_time, fee, success) 
             VALUES ($1, $2, $3, $4, $5) 
             ON CONFLICT (signature) DO NOTHING",
            &[&tx.signature, &(tx.slot as i64), &tx.block_time, &(tx.fee as i64), &tx.success]
        ).await.map_err(|e| IndexerError::InternalError(e.to_string()))?;

        for (idx, account) in tx.accounts.iter().enumerate() {
            db_tx.execute(
                "INSERT INTO transaction_accounts (transaction_signature, account_key, account_index) 
                 VALUES ($1, $2, $3) 
                 ON CONFLICT DO NOTHING",
                &[&tx.signature, account, &(idx as i32)]
            ).await.map_err(|e| IndexerError::InternalError(e.to_string()))?;
        }

        for (idx, ix) in tx.instructions.iter().enumerate() {
            let row = db_tx.query_one(
                "INSERT INTO instructions (transaction_signature, instruction_index, program_id, data, parent_index, is_inner) 
                 VALUES ($1, $2, $3, $4, $5, $6) 
                 RETURNING id",
                &[&tx.signature, &(idx as i32), &ix.program_id, &ix.data, &ix.parent_index.map(|i| i as i32), &ix.is_inner]
            ).await.map_err(|e| IndexerError::InternalError(e.to_string()))?;

            let instruction_id: i64 = row.get(0);

            for (acc_idx, account) in ix.accounts.iter().enumerate() {
                db_tx.execute(
                    "INSERT INTO instruction_accounts (instruction_id, account_key, account_index) 
                     VALUES ($1, $2, $3) 
                     ON CONFLICT DO NOTHING",
                    &[&instruction_id, account, &(acc_idx as i32)]
                ).await.map_err(|e| IndexerError::InternalError(e.to_string()))?;
            }
        }

        db_tx.commit().await.map_err(|e| IndexerError::InternalError(e.to_string()))?;
        Ok(())
    }
}
