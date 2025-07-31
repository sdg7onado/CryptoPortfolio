use crate::errors::PortfolioError;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

pub struct Database {
    pg_pool: Pool<Postgres>,
    redis_client: redis::Client,
}

#[derive(sqlx::FromRow)]
pub struct Trade {
    pub id: i32,
    pub symbol: String,
    pub quantity: f64,
    pub price: f64,
    pub action: String,
    pub timestamp: DateTime<Utc>,
}

impl Database {
    pub async fn new(postgres_url: &str, redis_url: &str) -> Result<Self, PortfolioError> {
        let pg_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(postgres_url)
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;

        let redis_client = redis::Client::open(redis_url)
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;

        // Initialize PostgreSQL table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS trades (
                id SERIAL PRIMARY KEY,
                symbol VARCHAR NOT NULL,
                quantity DOUBLE PRECISION NOT NULL,
                price DOUBLE PRECISION NOT NULL,
                action VARCHAR NOT NULL,
                timestamp TIMESTAMP WITH TIME ZONE NOT NULL
            )
            "#,
        )
        .execute(&pg_pool)
        .await
        .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;

        Ok(Database {
            pg_pool,
            redis_client,
        })
    }

    pub async fn log_trade(
        &self,
        symbol: &str,
        quantity: f64,
        price: f64,
        action: &str,
    ) -> Result<(), PortfolioError> {
        let timestamp = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO trades (symbol, quantity, price, action, timestamp)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(symbol)
        .bind(quantity)
        .bind(price)
        .bind(action)
        .bind(timestamp)
        .execute(&self.pg_pool)
        .await
        .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_cached_price(&self, symbol: &str) -> Result<Option<f64>, PortfolioError> {
        let mut conn = self
            .redis_client
            .get_async_connection()
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        let price: Option<f64> = conn
            .get(format!("price:{}", symbol))
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        Ok(price)
    }

    pub async fn cache_price(&self, symbol: &str, price: f64) -> Result<(), PortfolioError> {
        let mut conn = self
            .redis_client
            .get_async_connection()
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        conn.set_ex(&format!("price:{}", symbol), price, 300) // Cache for 5 minutes
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_cached_sentiment(&self, symbol: &str) -> Result<Option<f64>, PortfolioError> {
        let mut conn = self
            .redis_client
            .get_async_connection()
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        let sentiment: Option<f64> = conn
            .get(format!("sentiment:{}", symbol))
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        Ok(sentiment)
    }

    pub async fn cache_sentiment(
        &self,
        symbol: &str,
        sentiment: f64,
        ttl: u64,
    ) -> Result<(), PortfolioError> {
        let mut conn = self
            .redis_client
            .get_async_connection()
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        let ttl_usize: usize = ttl.try_into().map_err(|_| {
            PortfolioError::DatabaseError(format!("TTL value {} too large for usize", ttl))
        })?;
        conn.set_ex(format!("sentiment:{}", symbol), sentiment, ttl_usize)
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}

// Add method to get TTL from Redis (new)
impl Database {
    pub async fn get_cached_sentiment_ttl(
        &self,
        symbol: &str,
    ) -> Result<Option<u64>, PortfolioError> {
        let mut conn = self
            .redis_client
            .get_async_connection()
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        let ttl: Option<i64> = conn
            .ttl(format!("sentiment:{}", symbol))
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        Ok(ttl.map(|t| t as u64))
    }
}
