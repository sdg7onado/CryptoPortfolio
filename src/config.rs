use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub environment: String, // "dev" or "prod"
    pub exchanges: Vec<ExchangeConfig>,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub portfolio: PortfolioConfig,
}

#[derive(Deserialize, Debug)]
pub struct ExchangeConfig {
    pub name: String, // e.g., "coingecko", "binance"
    pub api_key: String,
    pub api_secret: String,
    pub base_url: String,
}

#[derive(Deserialize, Debug)]
pub struct DatabaseConfig {
    pub postgres_url: String,
}

#[derive(Deserialize, Debug)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct PortfolioConfig {
    pub check_interval_secs: u64,
    pub max_allocation: f64,       // e.g., 0.6 for 60%
    pub stop_loss_percentage: f64, // e.g., 0.2 for 20%
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}
