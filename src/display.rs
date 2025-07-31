use crate::database::Database;
use crate::errors::PortfolioError;
use crate::portfolio::Portfolio;
use comfy_table::{Cell, Color, Table};
use std::collections::HashMap;

pub fn display_portfolio(
    portfolio: &Portfolio,
    total_value: f64,
    sentiments: &HashMap<String, f64>,
) {
    let mut table = Table::new();
    table.set_header(vec![
        "Symbol",
        "Quantity",
        "Purchase Price",
        "Stop-Loss",
        "Current Value",
        "Sentiment",
    ]);
    for holding in &portfolio.holdings {
        let current_value = holding.quantity * sentiments.get(&holding.symbol).unwrap_or(&0.0);
        table.add_row(vec![
            holding.symbol.clone(),
            format!("{:.2}", holding.quantity),
            format!("${:.2}", holding.purchase_price),
            format!("${:.2}", holding.stop_loss),
            format!("${:.2}", current_value),
            format!("{:.2}", sentiments.get(&holding.symbol).unwrap_or(&0.5)),
        ]);
    }
    table.add_row(vec![
        "Cash".to_string(),
        format!("${:.2}", portfolio.cash),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
    ]);
    table.add_row(vec![
        "Total".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        format!("${:.2}", total_value),
        "".to_string(),
    ]);

    println!("=== Portfolio Status ===\n{}", table);
}

pub async fn display_sentiment_screen(
    portfolio: &Portfolio,
    sentiments: &HashMap<String, f64>,
    db: &Database,
    positive_threshold: f64,
    negative_threshold: f64,
    use_colors: bool,
) -> Result<(), PortfolioError> {
    let mut table = Table::new();
    table.set_header(vec![
        "Symbol",
        "Sentiment Score",
        "Data Source",
        "Cache TTL",
        "Recommendation",
    ]);
    for holding in &portfolio.holdings {
        let sentiment = *sentiments.get(&holding.symbol).unwrap_or(&0.5);
        let (source, ttl) =
            if let Some(cached_sentiment) = db.get_cached_sentiment(&holding.symbol).await? {
                (
                    "Redis Cache".to_string(),
                    db.get_cached_sentiment_ttl(&holding.symbol)
                        .await?
                        .unwrap_or(0),
                )
            } else {
                ("API Fetch".to_string(), 0)
            };
        let recommendation = if sentiment >= positive_threshold {
            "Hold/Buy".to_string()
        } else if sentiment <= negative_threshold {
            "Sell".to_string()
        } else {
            "Monitor".to_string()
        };
        let recommendation_cell = if use_colors {
            if sentiment >= positive_threshold {
                Cell::new(&recommendation).fg(Color::Green)
            } else if sentiment <= negative_threshold {
                Cell::new(&recommendation).fg(Color::Red)
            } else {
                Cell::new(&recommendation)
            }
        } else {
            Cell::new(&recommendation)
        };
        table.add_row(vec![
            Cell::new(holding.symbol.clone()),
            Cell::new(format!("{:.2}", sentiment)),
            Cell::new(source),
            Cell::new(format!("{}s", ttl)),
            recommendation_cell,
        ]);
    }

    println!(
        "=== Sentiment Analysis Dashboard ===\nTimestamp: {}\n{}",
        chrono::Utc::now(),
        table
    );
    Ok(())
}
