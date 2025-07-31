use reqwest::Client;
use serde::Deserialize;
use comfy_table::{Table, Cell, Color, Attribute};
use crate::errors::PortfolioError;
use crate::database::Database;
use chrono::Utc;

#[derive(Deserialize, Debug)]
pub struct MarketData {
    pub symbol: String,
    pub price: f64,
    pub market_cap: f64,
    pub price_change_24h: f64, // Percentage change
}

pub struct MarketProvider {
    client: Client,
    base_url: String,
    api_key: String,
}

impl MarketProvider {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        MarketProvider {
            client: Client::new(),
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn fetch_market_data(&self, symbols: &[String]) -> Result<Vec<MarketData>, PortfolioError> {
        // Mock CoinGecko API call for all coins (replace with real endpoint)
        let url = format!("{}/coins/markets?vs_currency=usd&per_page=100&page=1&key={}", self.base_url, self.api_key);
        let resp = self.client
            .get(&url)
            .header("User-Agent", "crypto_portfolio/0.1")
            .send()
            .await
            .map_err(|e| PortfolioError::ExchangeError(e.to_string()))?;
        let mut data: Vec<MarketData> = resp
            .json()
            .await
            .map_err(|e| PortfolioError::ExchangeError(e.to_string()))?;
        
        // Ensure pinned symbols (PHA, SUI, DUSK) are included
        for symbol in symbols {
            if !data.iter().any(|d| d.symbol == *symbol) {
                let price = self.fetch_single_price(symbol).await?;
                data.push(MarketData {
                    symbol: symbol.clone(),
                    price,
                    market_cap: 0.0, // Placeholder; fetch real market cap if needed
                    price_change_24h: 0.0,
                });
            }
        }
        Ok(data)
    }

    async fn fetch_single_price(&self, symbol: &str) -> Result<f64, PortfolioError> {
        let url = format!("{}/simple/price?ids={}&vs_currencies=usd&key={}", self.base_url, symbol, self.api_key);
        let resp: serde_json::Value = self.client
            .get(&url)
            .header("User-Agent", "crypto_portfolio/0.1")
            .send()
            .await
            .map_err(|e| PortfolioError::ExchangeError(e.to_string()))?
            .json()
            .await
            .map_err(|e| PortfolioError::ExchangeError(e.to_string()))?;
        let price = resp[symbol]["usd"]
            .as_f64()
            .ok_or_else(|| PortfolioError::ExchangeError("Price not found".to_string()))?;
        Ok(price)
    }
}

pub async fn display_market_screen(
    provider: &MarketProvider,
    db: &Database,
    pinned_symbols: &[String],
    sort_by: &str,
    use_colors: bool,
) -> Result<(), PortfolioError> {
    let mut market_data = provider.fetch_market_data(pinned_symbols).await?;
    
    // Sort data (default by market cap descending)
    match sort_by {
        "price_change_24h" => market_data.sort_by(|a, b| b.price_change_24h.partial_cmp(&a.price_change_24h).unwrap_or(std::cmp::Ordering::Equal)),
        _ => market_data.sort_by(|a, b| b.market_cap.partial_cmp(&a.market_cap).unwrap_or(std::cmp::Ordering::Equal)),
    }

    // Move pinned symbols to the top
    let mut pinned = Vec::new();
    let mut others = Vec::new();
    for data in market_data {
        if pinned_symbols.contains(&data.symbol) {
            pinned.push(data);
        } else {
            others.push(data);
        }
    }
    pinned.sort_by(|a, b| pinned_symbols.iter().position(|s| s == &a.symbol).cmp(&pinned_symbols.iter().position(|s| s == &b.symbol)));
    let final_data = [pinned, others].concat();

    // Cache prices in Redis
    for data in &final_data {
        db.cache_price(&data.symbol, data.price).await?;
    }

    // Create table
    let mut table = Table::new();
    table.set_header(vec![
        Cell::new("Symbol").add_attribute(Attribute::Bold),
        Cell::new("Price (USD)").add_attribute(Attribute::Bold),
        Cell::new("Market Cap (USD)").add_attribute(Attribute::Bold),
        Cell::new("24h Change (%)").add_attribute(Attribute::Bold),
    ]);

    for data in final_data {
        let price_color = if use_colors && data.price_change_24h > 0.0 { Color::Green } else if use_colors && data.price_change_24h < 0.0 { Color::Red } else { Color::White };
        let symbol_color = if pinned_symbols.contains(&data.symbol) && use_colors { Color::Green } else { Color::White };
        table.add_row(vec![
            Cell::new(&data.symbol).fg(symbol_color).add_attribute(if pinned_symbols.contains(&data.symbol) { Attribute::Bold } else { Attribute::Reset }),
            Cell::new(format!("${:.2}", data.price)).fg(price_color),
            Cell::new(format!("${:.0}", data.market_cap)),
            Cell::new(format!("{:.2}%", data.price_change_24h)).fg(price_color),
        ]);
    }

    println!("\n=== Live Market Updates ===");
    println!("Timestamp: {}", Utc::now().to_rfc3339());
    println!("{}", table);
    Ok(())
}