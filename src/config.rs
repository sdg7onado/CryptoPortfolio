use crate::errors::PortfolioError;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub environment: String, // "dev" or "prod"
    pub exchanges: Vec<ExchangeConfig>,
    pub marketprovider: ExchangeConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub portfolio: PortfolioConfig,
    pub sentiment: SentimentConfig,
    pub display: DisplayConfig,
    pub market: MarketConfig,
    pub notification: NotificationConfig,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ExchangeConfig {
    pub name: String, // e.g., "coingecko", "binance"
    pub api_key: String,
    pub api_secret: String,
    pub base_url: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DatabaseConfig {
    pub postgres_url: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PortfolioConfig {
    pub check_interval_secs: u64,
    pub max_allocation: f64,       // e.g., 0.6 for 60%
    pub stop_loss_percentage: f64, // e.g., 0.2 for 20%
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct SentimentConfig {
    pub api_url: String,
    pub api_key: String,
    pub cache_ttl_secs: u64,
    pub positive_threshold: f64,
    pub negative_threshold: f64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DisplayConfig {
    pub sentiment_refresh_secs: u64, // Refresh rate for sentiment screen
    pub use_colors: bool,            // Enable/disable color output
}

#[derive(Deserialize, Clone, Debug)]
pub struct MarketConfig {
    pub refresh_secs: u64,
    pub sort_by: String,             // e.g., "market_cap" or "price_change_24h"
    pub pinned_symbols: Vec<String>, // e.g., ["phala-network", "sui", "dusk-network"]
}

#[derive(Deserialize, Clone, Debug)]
pub struct NotificationConfig {
    pub sms_enabled: bool,
    pub email_enabled: bool,
    pub twilio_account_sid: String,
    pub twilio_auth_token: String,
    pub twilio_phone_number: String,
    pub recipient_phone_number: String,
    pub sendgrid_api_key: String,
    pub sender_email: String,
    pub recipient_email: String,
    pub notification_thresholds: NotificationThresholds,
}

#[derive(Deserialize, Clone, Debug)]
pub struct NotificationThresholds {
    pub portfolio_value_change_percent: f64,
    pub holding_value_change_percent: f64,
    pub sentiment_change: f64,
}

pub fn load_config() -> Result<Config, PortfolioError> {
    let config_str = fs::read_to_string("config.toml")
        .map_err(|e| PortfolioError::ConfigError(e.to_string()))?;
    let config: Config =
        toml::from_str(&config_str).map_err(|e| PortfolioError::ConfigError(e.to_string()))?;
    Ok(config)
}
