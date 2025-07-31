use sqlx::{Pool, Postgres};
use redis::AsyncCommands;
use crate::errors::PortfolioError;
use chrono::{DateTime, Utc};

pub struct Database {
    pg_pool: Pool<Postgres>,
    redis_client: redis::Client,
}

#[derive(sqlx::FromRow)]
pub struct Trade {
    pub symbol: String,
    pub quantity: f64,
    pub price: f64,
    pub action: String,
    pub timestamp: DateTime<Utc>,
}

impl Database {
    pub async fn new(pg_url: &str, redis_url: &str) -> Result<Self, PortfolioError> {
        let pg_pool = Pool::<Postgres>::connect(pg_url)
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        let redis_client = redis::Client::open(redis_url)
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS trades (
                id SERIAL PRIMARY KEY,
                symbol VARCHAR NOT NULL,
                quantity FLOAT NOT NULL,
                price FLOAT NOT NULL,
                action VARCHAR NOT NULL,
                timestamp TIMESTAMP NOT NULL
            )"
        )
        .execute(&pg_pool)
        .await
        .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        Ok(Database { pg_pool, redis_client })
    }

    pub async fn cache_price(&self, symbol: &str, price: f64) -> Result<(), PortfolioError> {
        let mut conn = self.redis_client
            .get_async_connection()
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        conn.set_ex(&format!("price:{}", symbol), price, 300) // Cache for 5 minutes
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_cached_price(&self, symbol: &str) -> Result<Option<f64>, PortfolioError> {
        let mut conn = self.redis_client
            .get_async_connection()
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        let price: Option<f64> = conn
            .get(&format!("price:{}", symbol))
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        Ok(price)
    }

    pub async fn log_trade(&self, trade: Trade) -> Result<(), PortfolioError> {
        sqlx::query(
            "INSERT INTO trades (symbol, quantity, price, action, timestamp) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(trade.symbol)
        .bind(trade.quantity)
        .bind(trade.price)
        .bind(trade.action)
        .bind(trade.timestamp)
        .execute(&self.pg_pool)
        .await
        .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}