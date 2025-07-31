use crate::config::load_config;
use crate::database::Database;
use crate::display::{display_portfolio, display_sentiment_screen};
use crate::errors::PortfolioError;
use crate::exchange::Exchange;
use crate::exchange::SentimentProvider;
use crate::exchange::{create_exchange, create_sentiment_provider};
use crate::logger::{init_logger, log_action};
use crate::market::{display_market_screen, MarketProvider};
use crate::notification::Notifier;
use crate::portfolio::Portfolio;
use dotenv::dotenv;
use std::collections::HashMap;
use std::process::Command;
use tokio::time::{sleep, Duration};

mod config;
mod database;
mod display;
mod errors;
mod exchange;
mod logger;
mod market;
mod notification;
mod portfolio;

#[tokio::main]
async fn portfolio_screen() -> Result<(), PortfolioError> {
    let config = load_config()?;
    init_logger(&config.environment)?;
    let db = Database::new(&config.database.postgres_url, &config.redis.url).await?;
    let exchange = create_exchange(&config.exchanges[0]); // Returns CoinGeckoExchange
    let sentiment_provider =
        create_sentiment_provider(&config.sentiment.api_url, &config.sentiment.api_key); // Returns LunarCrushProvider
    let notifier = Notifier::new(config.notification.clone());
    let mut portfolio = Portfolio::new(config.portfolio.clone());
    let mut previous_value = 0.0;
    let mut previous_prices = HashMap::new();
    let mut previous_sentiments = HashMap::new();

    loop {
        let mut sentiments = HashMap::new();
        let mut current_prices = HashMap::new();
        for holding in &portfolio.holdings {
            if let Some(cached_price) = db.get_cached_price(&holding.symbol).await? {
                log_action(
                    &format!(
                        "{}: Using cached price ${:.2}",
                        holding.symbol, cached_price
                    ),
                    &config.environment,
                )?;
                current_prices.insert(holding.symbol.clone(), cached_price);
            } else {
                let price = exchange.fetch_price(&holding.symbol).await?;
                db.cache_price(&holding.symbol, price).await?;
                log_action(
                    &format!("{}: Fetched price ${:.2}", holding.symbol, price),
                    &config.environment,
                )?;
                current_prices.insert(holding.symbol.clone(), price);
            }
            if let Some(cached_sentiment) = db.get_cached_sentiment(&holding.symbol).await? {
                sentiments.insert(holding.symbol.clone(), cached_sentiment);
                log_action(
                    &format!(
                        "{}: Using cached sentiment {:.2}",
                        holding.symbol, cached_sentiment
                    ),
                    &config.environment,
                )?;
            } else {
                let sentiment = sentiment_provider.fetch_sentiment(&holding.symbol).await?;
                db.cache_sentiment(&holding.symbol, sentiment, config.sentiment.cache_ttl_secs)
                    .await?;
                sentiments.insert(holding.symbol.clone(), sentiment);
                log_action(
                    &format!("{}: Fetched sentiment {:.2}", holding.symbol, sentiment),
                    &config.environment,
                )?;
            }
        }

        let total_value = portfolio
            .check_portfolio(
                &exchange,
                &sentiment_provider,
                &db,
                &notifier,
                config.sentiment.negative_threshold,
                previous_value,
                &previous_prices,
                &previous_sentiments,
            )
            .await?;

        previous_value = total_value;
        previous_prices = current_prices.clone();
        previous_sentiments = sentiments.clone();

        display_portfolio(&portfolio, total_value, &sentiments);
        log_action(
            &format!("Portfolio value: ${:.2}", total_value),
            &config.environment,
        )?;

        sleep(Duration::from_secs(config.portfolio.check_interval_secs)).await;
    }
}

#[tokio::main]
async fn sentiment_screen() -> Result<(), PortfolioError> {
    let config = load_config()?;
    init_logger(&config.environment)?;
    let db = Database::new(&config.database.postgres_url, &config.redis.url).await?;
    let sentiment_provider =
        create_sentiment_provider(&config.sentiment.api_url, &config.sentiment.api_key);
    let portfolio = Portfolio::new(config.portfolio.clone());

    loop {
        let mut sentiments = HashMap::new();
        for holding in &portfolio.holdings {
            if let Some(cached_sentiment) = db.get_cached_sentiment(&holding.symbol).await? {
                sentiments.insert(holding.symbol.clone(), cached_sentiment);
                log_action(
                    &format!(
                        "{}: Using cached sentiment {:.2}",
                        holding.symbol, cached_sentiment
                    ),
                    &config.environment,
                )?;
            } else {
                let sentiment = sentiment_provider.fetch_sentiment(&holding.symbol).await?;
                db.cache_sentiment(&holding.symbol, sentiment, config.sentiment.cache_ttl_secs)
                    .await?;
                sentiments.insert(holding.symbol.clone(), sentiment);
                log_action(
                    &format!("{}: Fetched sentiment {:.2}", holding.symbol, sentiment),
                    &config.environment,
                )?;
            }
        }

        display_sentiment_screen(
            &portfolio,
            &sentiments,
            &db,
            config.sentiment.positive_threshold,
            config.sentiment.negative_threshold,
            config.display.use_colors,
        )
        .await?;

        sleep(Duration::from_secs(config.display.sentiment_refresh_secs)).await;
    }
}

#[tokio::main]
async fn market_screen() -> Result<(), PortfolioError> {
    let config = load_config()?;
    init_logger(&config.environment)?;
    let db = Database::new(&config.database.postgres_url, &config.redis.url).await?;
    let market_provider =
        MarketProvider::new(&config.exchanges[0].base_url, &config.exchanges[0].api_key);

    loop {
        display_market_screen(
            &market_provider,
            &db,
            &config.market.pinned_symbols,
            &config.market.sort_by,
            config.display.use_colors,
        )
        .await?;

        sleep(Duration::from_secs(config.market.refresh_secs)).await;
    }
}

fn main() {
    dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "portfolio" => portfolio_screen().expect("Portfolio screen failed"),
            "sentiment" => sentiment_screen().expect("Sentiment screen failed"),
            "market" => market_screen().expect("Market screen failed"),
            _ => eprintln!("Invalid subcommand. Use 'portfolio', 'sentiment', or 'market'."),
        }
    } else {
        let config = load_config().expect("Failed to load config");
        init_logger(&config.environment).expect("Failed to initialize logger");

        // Detect terminal emulator for Linux
        let (terminal_cmd, terminal_args) = if cfg!(target_os = "windows") {
            (
                "cmd",
                vec!["/C", "start", "cmd", "/K", "cargo", "run", "--"],
            )
        } else {
            // Try common Linux terminal emulators
            let terminals = [
                ("xterm", vec!["-e", "cargo", "run", "--"]),
                ("gnome-terminal", vec!["--", "cargo", "run", "--"]),
                ("konsole", vec!["-e", "cargo", "run", "--"]),
            ];
            terminals
                .into_iter()
                .find(|(cmd, _)| Command::new(cmd).arg("--version").output().is_ok())
                .unwrap_or_else(|| {
                    eprintln!("No terminal emulator found (xterm, gnome-terminal, konsole). Falling back to xterm.");
                    ("xterm", vec!["-e", "cargo", "run", "--"])
                })
        };

        // Spawn console windows for each screen
        for screen in ["portfolio", "sentiment", "market"] {
            if let Err(e) = Command::new(terminal_cmd)
                .args(&terminal_args)
                .arg(screen)
                .spawn()
            {
                eprintln!("Failed to spawn {} screen: {}", screen, e);
            }
        }

        // Keep the main process alive
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
