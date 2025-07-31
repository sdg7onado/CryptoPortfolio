use comfy_table::{Table, Cell, Color, Attribute};
use crate::portfolio::Portfolio;
use std::collections::HashMap;
use chrono::{Utc, Duration};
use redis::AsyncCommands;
use crate::database::Database;
use crate::errors::PortfolioError;

pub fn display_portfolio(portfolio: &Portfolio, total_value: f64, sentiments: &HashMap<String, f64>) {
    let mut table = Table::new();
    table.set_header(vec!["Symbol", "Quantity", "Purchase Price", "Stop-Loss", "Current Value", "Sentiment"]);
    
    for holding in &portfolio.holdings {
        let current_value = holding.quantity * holding.purchase_price; // Placeholder; update with real price
        let sentiment = sentiments.get(&holding.symbol).copied().unwrap_or(0.5);
        let sentiment_color = if sentiment >= 0.7 { Color::Green } else if sentiment <= 0.3 { Color::Red } else { Color::Yellow };
        table.add_row(vec![
            Cell::new(&holding.symbol).fg(Color::Green),
            Cell::new(format!("{:.2}", holding.quantity)),
            Cell::new(format!("${:.2}", holding.purchase_price)),
            Cell::new(format!("${:.2}", holding.stop_loss)),
            Cell::new(format!("${:.2}", current_value)).fg(Color::Cyan),
            Cell::new(format!("{:.2}", sentiment)).fg(sentiment_color),
        ]);
    }
    
    table.add_row(vec![
        Cell::new("Cash").fg(Color::Yellow),
        Cell::new(format!("${:.2}", portfolio.cash)),
        Cell::new(""),
        Cell::new(""),
        Cell::new(""),
        Cell::new(""),
    ]);
    
    table.add_row(vec![
        Cell::new("Total").fg(Color::White).add_attribute(Attribute::Bold),
        Cell::new(""),
        Cell::new(""),
        Cell::new(""),
        Cell::new(format!("${:.2}", total_value)).fg(Color::White).add_attribute(Attribute::Bold),
        Cell::new(""),
    ]);
    
    println!("\n=== Portfolio Status ===");
    println!("{}", table);
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
        Cell::new("Symbol").add_attribute(Attribute::Bold),
        Cell::new("Sentiment Score").add_attribute(Attribute::Bold),
        Cell::new("Data Source").add_attribute(Attribute::Bold),
        Cell::new("Cache TTL").add_attribute(Attribute::Bold),
        Cell::new("Recommendation").add_attribute(Attribute::Bold),
    ]);

    for holding in &portfolio.holdings {
        let sentiment = sentiments.get(&holding.symbol).copied().unwrap_or(0.5);
        let sentiment_color = if use_colors {
            if sentiment >= positive_threshold { Color::Green }
            else if sentiment <= negative_threshold { Color::Red }
            else { Color::Yellow }
        } else {
            Color::White
        };

        // Fetch cache TTL from Redis
        let mut redis_conn = db.redis_client
            .get_async_connection()
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        let ttl: Option<i64> = redis_conn
            .ttl(format!("sentiment:{}", holding.symbol))
            .await
            .map_err(|e| PortfolioError::DatabaseError(e.to_string()))?;
        let ttl_str = ttl.map_or("N/A".to_string(), |t| {
            let duration = Duration::seconds(t);
            format!("{}s", duration.num_seconds())
        });

        // Determine data source
        let source = if db.get_cached_sentiment(&holding.symbol).await?.is_some() {
            "Redis Cache"
        } else {
            "API Fetch"
        };

        // Generate recommendation
        let recommendation = if sentiment >= positive_threshold {
            "Hold/Buy"
        } else if sentiment <= negative_threshold {
            "Sell"
        } else {
            "Monitor"
        };
        let recommendation_color = if use_colors {
            if sentiment >= positive_threshold { Color::Green }
            else if sentiment <= negative_threshold { Color::Red }
            else { Color::Yellow }
        } else {
            Color::White
        };

        table.add_row(vec![
            Cell::new(&holding.symbol).fg(Color::Green),
            Cell::new(format!("{:.2}", sentiment)).fg(sentiment_color),
            Cell::new(source).fg(Color::Cyan),
            Cell::new(ttl_str).fg(Color::White),
            Cell::new(recommendation).fg(recommendation_color),
        ]);
    }

    println!("\n=== Sentiment Analysis Dashboard ===");
    println!("Timestamp: {}", Utc::now().to_rfc3339());
    println!("{}", table);
    Ok(())
}