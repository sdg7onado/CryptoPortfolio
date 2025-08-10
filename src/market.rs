use std::str::FromStr;

use crate::errors::PortfolioError;
use crate::exchange::{BinanceExchange, Exchange};
use comfy_table::{Cell, Color, Table};
use icu::decimal::input::Decimal;
use icu::decimal::DecimalFormatter;
use icu::locale::{locale, Locale};
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
    pub price_change_percentage_24h: f64,
    pub high_24h: f64,
    pub low_24h: f64,
    pub total_volume: f64,
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
                    price_change_percentage_24h: 0.0,
                    high_24h: 0.0,
                    low_24h: 0.0,
                    total_volume: 0.0,
                });
            }
        }
        Ok(data)
    }
}

pub async fn display_market_screen<'a>(
    market_provider: &MarketProvider<'a>,
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
        "S/N",
        "Symbol",
        "Price (USD)",
        "Market Cap (USD)",
        "24h Change (USD)",
        "24h Change (%)",
        "High (24h)",
        "Low (24h)",
        "Total Volume (24h)",
    ]);
    for (i, data) in final_data.iter().enumerate() {
        table.add_row(vec![
            Cell::new(i + 1),
            Cell::new(data.symbol.to_uppercase()),
            Cell::new(format!("${}", format_number(data.price, None))),
            Cell::new(format!("${}", format_number(data.market_cap, None))),
            set_cell_color(data.price_change_24h, use_colors, false),
            set_cell_color(data.price_change_percentage_24h, use_colors, true),
            Cell::new(format!("{}", format_number(data.high_24h, None))),
            Cell::new(format!("{}", format_number(data.low_24h, None))),
            Cell::new(format!("${}", format_number(data.total_volume, None))),
        ]);
    }

    println!(
        "=== Live Market Updates ===\nTimestamp: {}\n{}",
        chrono::Utc::now(),
        table
    );
    Ok(())
}

fn set_cell_color(amount: f64, use_colors: bool, use_percentage: bool) -> Cell {
    let percent = if use_percentage { "%" } else { "" };
    let change = format!("{}{}", format_number(amount, None), percent);
    let change_cell = if use_colors {
        if amount > 0.0 {
            Cell::new(&change).fg(Color::Green)
        } else {
            Cell::new(&change).fg(Color::Red)
        }
    } else {
        Cell::new(&change)
    };
    change_cell
}

fn format_number(amount: f64, locale: Option<Locale>) -> String {
    let locale = locale.unwrap_or(locale!("en-US"));

    let formatter = DecimalFormatter::try_new(locale.into(), Default::default())
        .expect("locale should be present");

    let decimal = Decimal::from_str(&amount.to_string()).unwrap();
    formatter.format(&decimal).to_string()
}
