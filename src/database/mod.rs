use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use crate::error::Result;
use crate::config::Config;

pub struct Database {
    pub pool: Pool<Postgres>,
}

impl Database {
    pub async fn new(config: &Config) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.database_url) 
            .await?;

        Ok(Database { pool })
    }
}
