use crate::database::Database;
use crate::errors::PortfolioError;
use crate::exchange::{DetailedSentiment, SentimentProvider};
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
    sentiment_provider: &impl SentimentProvider,
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
        "Daily Avg",
        "1-Week",
        "1-Month",
    ]);
    for holding in &portfolio.holdings {
        let sentiment = *sentiments.get(&holding.symbol).unwrap_or(&0.5);
        let detailed = sentiment_provider
            .fetch_detailed_sentiment(&holding.symbol)
            .await?;
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
            Cell::new(format!("{:.2}", detailed.daily_average)),
            Cell::new(format!(
                "{:.2} ({:.0}%)",
                detailed.one_week_value,
                detailed.one_week_change * 100.0
            )),
            Cell::new(format!(
                "{:.2} ({:.0}%)",
                detailed.one_month_value,
                detailed.one_month_change * 100.0
            )),
        ]);
    }

    println!(
        "=== Sentiment Analysis Dashboard ===\nTimestamp: {}\n{}",
        chrono::Utc::now(),
        table
    );

    // Detailed sentiment for each holding
    for holding in &portfolio.holdings {
        let detailed = sentiment_provider
            .fetch_detailed_sentiment(&holding.symbol)
            .await?;

        // High/Low table
        let mut high_low_table = Table::new();
        high_low_table.set_header(vec!["1-Year High", "Date", "1-Year Low", "Date"]);
        high_low_table.add_row(vec![
            format!("{:.2}", detailed.one_year_high),
            detailed.one_year_high_date,
            format!("{:.2}", detailed.one_year_low),
            detailed.one_year_low_date,
        ]);
        println!("\n{} High/Low:", holding.symbol);
        println!("{}", high_low_table);

        // Supportive Themes table
        let mut supportive_table = Table::new();
        supportive_table.set_header(vec!["Supportive Theme", "Weight", "Description"]);
        for theme in detailed.supportive_themes {
            supportive_table.add_row(vec![
                theme.name,
                format!("{:.0}%", theme.weight * 100.0),
                theme.description,
            ]);
        }
        println!("\n{} Supportive Themes:", holding.symbol);
        println!("{}", supportive_table);

        // Critical Themes table
        let mut critical_table = Table::new();
        critical_table.set_header(vec!["Critical Theme", "Weight", "Description"]);
        for theme in detailed.critical_themes {
            critical_table.add_row(vec![
                theme.name,
                format!("{:.0}%", theme.weight * 100.0),
                theme.description,
            ]);
        }
        println!("\n{} Critical Themes:", holding.symbol);
        println!("{}", critical_table);

        // Network Engagement table
        let mut engagement_table = Table::new();
        engagement_table.set_header(vec![
            "Network",
            "Positive",
            "Positive %",
            "Neutral",
            "Neutral %",
            "Negative",
            "Negative %",
        ]);
        for (network, engagement) in detailed.network_engagement {
            engagement_table.add_row(vec![
                network,
                engagement.positive.to_string(),
                format!("{:.0}%", engagement.positive_percentage * 100.0),
                engagement.neutral.to_string(),
                format!("{:.0}%", engagement.neutral_percentage * 100.0),
                engagement.negative.to_string(),
                format!("{:.0}%", engagement.negative_percentage * 100.0),
            ]);
        }
        println!("\n{} Network Engagement:", holding.symbol);
        println!("{}", engagement_table);
    }

    Ok(())
}
