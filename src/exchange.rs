use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use crate::config::ExchangeConfig;
use crate::errors::PortfolioError;

#[derive(Deserialize)]
struct PriceResponse {
    price: f64,
}

pub trait Exchange {
    async fn fetch_price(&self, symbol: &str) -> Result<f64, PortfolioError>;
}

pub struct CoinGeckoExchange {
    client: Client,
    base_url: String,
}

impl CoinGeckoExchange {
    pub fn new(config: &ExchangeConfig) -> Self {
        CoinGeckoExchange {
            client: Client::new(),
            base_url: config.base_url.clone(),
        }
    }
}

impl Exchange for CoinGeckoExchange {
    async fn fetch_price(&self, symbol: &str) -> Result<f64, PortfolioError> {
        let url = format!("{}/simple/price?ids={}&vs_currencies=usd", self.base_url, symbol);
        let resp = self.client
            .get(&url)
            .header("User-Agent", "crypto_portfolio/0.1")
            .send()
            .await
            .map_err(|e| PortfolioError::ExchangeError(e.to_string()))?;
        let json: HashMap<String, HashMap<String, f64>> = resp
            .json()
            .await
            .map_err(|e| PortfolioError::ExchangeError(e.to_string()))?;
        let price = json
            .get(symbol)
            .and_then(|m| m.get("usd"))
            .copied()
            .ok_or_else(|| PortfolioError::ExchangeError("Price not found".to_string()))?;
        Ok(price)
    }
}

// Add more exchanges (e.g., Binance) by implementing the Exchange trait
pub fn create_exchange(config: &ExchangeConfig) -> Box<dyn Exchange> {
    match config.name.as_str() {
        "coingecko" => Box::new(CoinGeckoExchange::new(config)),
        _ => panic!("Unsupported exchange: {}", config.name),
    }
}