use crate::database::Database;
use crate::errors::PortfolioError;
use comfy_table::{Cell, Color, Table};
use reqwest::Client;

#[derive(Debug, Clone)] // From previous fix
pub struct MarketData {
    pub symbol: String,
    pub price: f64,
    pub market_cap: f64,
    pub price_change_24h: f64,
}

pub struct MarketProvider {
    client: Client,
    api_url: String,
    api_key: String,
}

impl MarketProvider {
    pub fn new(api_url: &str, api_key: &str) -> Self {
        MarketProvider {
            client: Client::new(),
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn fetch_market_data(
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

pub async fn display_market_screen(
    market_provider: &MarketProvider,
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
            Cell::new(format!("${:.2}", data.price)),
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
