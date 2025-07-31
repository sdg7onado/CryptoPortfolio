use thiserror::Error;

#[derive(Error, Debug)]
pub enum PortfolioError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Exchange API error: {0}")]
    ExchangeError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("IO error: {0}")]
    IoError(String),
}