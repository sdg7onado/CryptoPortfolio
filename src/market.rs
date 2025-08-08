use crate::database::Database;
use crate::errors::PortfolioError;
use crate::exchange::{BinanceExchange, Exchange};
use comfy_table::{Cell, Color, Table};
use icu::decimal::{FixedDecimal, FixedDecimalFormatter};
use icu::locid::Locale as IcuLocale;
use icu_testdata::get_provider;
use locale_config::Locale as SystemLocale;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketData {
    pub symbol: String,
    #[serde(rename = "current_price")]
    pub price: f64,
    pub market_cap: f64,
    pub price_change_24h: f64,
}

pub struct MarketProvider<'a> {
    client: Client,
    api_url: String,
    api_key: String,
    exchange: &'a BinanceExchange,
}

impl<'a> MarketProvider<'a> {
    pub fn new(api_url: &str, api_key: &str, exchange: &'a BinanceExchange) -> Self {
        MarketProvider {
            client: Client::new(),
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
            exchange: exchange,
        }
    }

    pub async fn fetch_market_data(
        &self,
        symbols: &[String],
    ) -> Result<Vec<MarketData>, PortfolioError> {
        let url = format!(
            "{}/coins/markets?vs_currency=usd&per_page=1000&page=1",
            self.api_url
        );
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("crypto_portfolio/0.1"));
        headers.insert(
            "x-cg-demo-api-key",
            HeaderValue::from_str(&self.api_key)
                .map_err(|e| PortfolioError::ExchangeError(e.to_string()))?,
        );
        let resp = self
            .client
            .get(&url)
            .headers(headers)
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
                //let price = self.exchange.fetch_single_price(symbol).await?;
                let price = self.exchange.fetch_price(symbol).await?;
                data.push(MarketData {
                    symbol: symbol.clone(),
                    price,
                    market_cap: 0.0,
                    price_change_24h: 0.0,
                });
            }
        }
        Ok(data)
    }

    /*     async fn fetch_single_price(&self, symbol: &str) -> Result<f64, PortfolioError> {
        let exchange = create_exchange(&config.exchanges[0]);
        let url = format!(
            //"{}/simple/price?ids={}&vs_currencies=usd&key={}USDT",
            "{}/api/v3/avgPrice?symbol={}USDT",
            self.exchange.api_url, symbol
        );
        let _ = log_action(&format!("Got here2 {} ", url), None);
        let resp: serde_json::Value = self
            .client
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
    } */

    pub async fn fetch_market_data_mock(
        &self,
        symbols: &[String],
    ) -> Result<Vec<MarketData>, PortfolioError> {
        // Mock data for testing
        Ok(vec![
            MarketData {
                symbol: "phala-network".to_string(),
                price: 0.22,
                market_cap: 150_000_000.0,
                price_change_24h: 10.0,
            },
            MarketData {
                symbol: "sui".to_string(),
                price: 3.10,
                market_cap: 250_000_000.0,
                price_change_24h: 3.33,
            },
            MarketData {
                symbol: "dusk-network".to_string(),
                price: 0.24,
                market_cap: 100_000_000.0,
                price_change_24h: -4.0,
            },
            MarketData {
                symbol: "bitcoin".to_string(),
                price: 118_050.85,
                market_cap: 2_300_000_000_000.0,
                price_change_24h: 2.5,
            },
            MarketData {
                symbol: "ethereum".to_string(),
                price: 3_500.00,
                market_cap: 420_000_000_000.0,
                price_change_24h: -1.2,
            },
            MarketData {
                symbol: "solana".to_string(),
                price: 180.00,
                market_cap: 80_000_000_000.0,
                price_change_24h: 5.0,
            },
        ])
    }
}

pub async fn display_market_screen<'a>(
    market_provider: &MarketProvider<'a>,
    db: &Database,
    pinned_symbols: &[String],
    sort_by: &str,
    use_colors: bool,
) -> Result<(), PortfolioError> {
    let market_data = market_provider.fetch_market_data(pinned_symbols).await?;

    // Split into pinned and others
    let pinned: Vec<MarketData> = market_data
        .iter()
        .filter(|data| pinned_symbols.contains(&data.symbol))
        .cloned()
        .collect();
    let others: Vec<MarketData> = market_data
        .into_iter()
        .filter(|data| !pinned_symbols.contains(&data.symbol))
        .collect();

    // Sort others by specified criterion
    let mut others = others;
    match sort_by {
        "market_cap" => others.sort_by(|a, b| b.market_cap.partial_cmp(&a.market_cap).unwrap()),
        "price_change_24h" => {
            others.sort_by(|a, b| b.price_change_24h.partial_cmp(&a.price_change_24h).unwrap())
        }
        _ => others.sort_by(|a, b| b.market_cap.partial_cmp(&a.market_cap).unwrap()),
    }

    // Combine pinned and others
    let final_data = [pinned, others].concat();

    let mut table = Table::new();
    table.set_header(vec![
        "Symbol",
        "Price (USD)",
        "Market Cap (USD)",
        "24h Change (%)",
    ]);
    for data in final_data {
        let change = format!("{:.2}%", data.price_change_24h); // Store as String
        let change_cell = if use_colors {
            if data.price_change_24h > 0.0 {
                Cell::new(&change).fg(Color::Green)
            } else {
                Cell::new(&change).fg(Color::Red)
            }
        } else {
            Cell::new(&change)
        };
        table.add_row(vec![
            Cell::new(data.symbol),
            Cell::new(format!(
                "${:.2}",
                format_price(data.price, &SystemLocale::en)
            )),
            Cell::new(format!("${:.0}", data.market_cap)),
            change_cell,
        ]);
    }

    println!(
        "=== Live Market Updates ===\nTimestamp: {}\n{}",
        chrono::Utc::now(),
        table
    );
    Ok(())
}

pub fn format_price(price: f64, locale: &SystemLocale) -> String {
    let provider = get_provider().expect("Failed to get ICU data provider");
    let formatter = FixedDecimalFormatter::try_new(
        &provider,
        IcuLocale::from(locale.to_string()),
        FixedDecimal::from(price),
    )
    .expect("Failed to create FixedDecimalFormatter");
    formatter.format().to_string()
}
