use dotenv::dotenv;
use tokio::time::{sleep, Duration};
use std::process::Command;
use std::collections::HashMap;
use crate::config::load_config;
use crate::exchange::{create_exchange, create_sentiment_provider};
use crate::market::{MarketProvider, display_market_screen};
use crate::portfolio::{Portfolio, PortfolioManager};
use crate::database::Database;
use crate::logger::{init_logger, log_action};
use crate::display::{display_portfolio, display_sentiment_screen};
use crate::notification::Notifier;
use crate::errors::PortfolioError;

mod config;
mod exchange;
mod portfolio;
mod database;
mod logger;
mod display;
mod market;
mod notification;
mod errors;

#[tokio::main]
async fn main() -> Result<(), PortfolioError> {
    dotenv().ok();
    let config = load_config()?;
    init_logger(&config.environment);

    let (terminal_cmd, terminal_args) = if cfg!(target_os = "windows") {
        ("cmd", vec!["/C", "cargo", "run", "--"])
    } else {
        ("xterm", vec!["-e", "cargo", "run", "--"])
    };

    Command::new(terminal_cmd)
        .args(&terminal_args)
        .arg("portfolio")
        .spawn()
        .map_err(|e| PortfolioError::IoError(e.to_string()))?;
    Command::new(terminal_cmd)
        .args(&terminal_args)
        .arg("sentiment")
        .spawn()
        .map_err(|e| PortfolioError::IoError(e.to_string()))?;
    Command::new(terminal_cmd)
        .args(&terminal_args)
        .arg("market")
        .spawn()
        .map_err(|e| PortfolioError::IoError(e.to_string()))?;

    Ok(())
}

#[tokio::main]
async fn portfolio_screen() -> Result<(), PortfolioError> {
    let config = load_config()?;
    init_logger(&config.environment);
    let db = Database::new(&config.database.postgres_url, &config.redis.url).await?;
    let exchange = create_exchange(&config.exchanges[0]);
    let sentiment_provider = create_sentiment_provider(&config.sentiment.api_url, &config.sentiment.api_key);
    let notifier = Notifier::new(config.notification.clone());
    let mut portfolio = Portfolio {
        holdings: vec![
            portfolio::Holding {
                symbol: "phala-network".to_string(),
                quantity: 250.0,
                purchase_price: 0.20,
                stop_loss: 0.16,
            },
            portfolio::Holding {
                symbol: "sui".to_string(),
                quantity: 10.0,
                purchase_price: 3.00,
                stop_loss: 2.40,
            },
            portfolio::Holding {
                symbol: "dusk-network".to_string(),
                quantity: 80.0,
                purchase_price: 0.25,
                stop_loss: 0.20,
            },
        ],
        cash: 0.0,
        previous_value: 0.0,
        previous_prices: HashMap::new(),
        previous_sentiments: HashMap::new(),
    };

    loop {
        let mut sentiments = HashMap::new();
        let mut current_prices = HashMap::new();
        for holding in &portfolio.holdings {
            if let Some(cached_price) = db.get_cached_price(&holding.symbol).await? {
                log_action(&format!("{}: Using cached price ${:.2}", holding.symbol, cached_price), &config.environment);
                current_prices.insert(holding.symbol.clone(), cached_price);
            } else {
                let price = exchange.fetch_price(&holding.symbol).await?;
                db.cache_price(&holding.symbol, price).await?;
                log_action(&format!("{}: Fetched price ${:.2}", holding.symbol, price), &config.environment);
                current_prices.insert(holding.symbol.clone(), price);
            }
            if let Some(cached_sentiment) = db.get_cached_sentiment(&holding.symbol).await? {
                sentiments.insert(holding.symbol.clone(), cached_sentiment);
                log_action(&format!("{}: Using cached sentiment {:.2}", holding.symbol, cached_sentiment), &config.environment);
            } else {
                let sentiment = sentiment_provider.fetch_sentiment(&holding.symbol).await?;
                db.cache_sentiment(&holding.symbol, sentiment, config.sentiment.cache_ttl_secs).await?;
                sentiments.insert(holding.symbol.clone(), sentiment);
                log_action(&format!("{}: Fetched sentiment {:.2}", holding.symbol, sentiment), &config.environment);
            }
        }

        // Check for sentiment changes
        for holding in &portfolio.holdings {
            if let Some(prev_sentiment) = portfolio.previous_sentiments.get(&holding.symbol) {
                if let Some(curr_sentiment) = sentiments.get(&holding.symbol) {
                    notifier.notify_sentiment_change(&holding.symbol, *prev_sentiment, *curr_sentiment).await?;
                }
            }
            portfolio.previous_sentiments.insert(holding.symbol.clone(), *sentiments.get(&holding.symbol).unwrap_or(&0.5));
        }

        // Check stop-losses
        let stop_loss_actions = portfolio.check_stop_loss(&*exchange, &*sentiment_provider, &db, config.sentiment.negative_threshold).await?;
        for action in stop_loss_actions {
            log_action(&action, &config.environment);
            println!("{}", action);
            notifier.notify_significant_action(&action).await?;
        }

        // Rebalance
        let rebalance_actions = portfolio.rebalance(&*exchange, &*sentiment_provider, &db, config.portfolio.max_allocation, config.sentiment.positive_threshold).await?;
        for action in rebalance_actions {
            log_action(&action, &config.environment);
            println!("{}", action);
            notifier.notify_significant_action(&action).await?;
        }

        // Check portfolio value changes
        let total_value = portfolio.get_value(&*exchange).await?;
        if portfolio.previous_value > 0.0 {
            notifier.notify_major_change(&portfolio, portfolio.previous_value, total_value, &portfolio.previous_prices, &current_prices).await?;
        }
        portfolio.previous_value = total_value;
        portfolio.previous_prices = current_prices.clone();

        // Display portfolio
        display_portfolio(&portfolio, total_value, &sentiments);
        log_action(&format!("Portfolio value: ${:.2}", total_value), &config.environment);

        sleep(Duration::from_secs(config.portfolio.check_interval_secs)).await;
    }
}

#[tokio::main]
async fn sentiment_screen() -> Result<(), PortfolioError> {
    let config = load_config()?;
    init_logger(&config.environment);
    let db = Database::new(&config.database.postgres_url, &config.redis.url).await?;
    let sentiment_provider = create_sentiment_provider(&config.sentiment.api_url, &config.sentiment.api_key);
    let portfolio = Portfolio {
        holdings: vec![
            portfolio::Holding {
                symbol: "phala-network".to_string(),
                quantity: 250.0,
                purchase_price: 0.20,
                stop_loss: 0.16,
            },
            portfolio::Holding {
                symbol: "sui".to_string(),
                quantity: 10.0,
                purchase_price: 3.00,
                stop_loss: 2.40,
            },
            portfolio::Holding {
                symbol: "dusk-network".to_string(),
                quantity: 80.0,
                purchase_price: 0.25,
                stop_loss: 0.20,
            },
        ],
        cash: 0.0,
        previous_value: 0.0,
        previous_prices: HashMap::new(),
        previous_sentiments: HashMap::new(),
    };

    loop {
        let mut sentiments = HashMap::new();
        for holding in &portfolio.holdings {
            if let Some(cached_sentiment) = db.get_cached_sentiment(&holding.symbol).await? {
                sentiments.insert(holding.symbol.clone(), cached_sentiment);
                log_action(&format!("{}: Using cached sentiment {:.2}", holding.symbol, cached_sentiment), &config.environment);
            } else {
                let sentiment = sentiment_provider.fetch_sentiment(&holding.symbol).await?;
                db.cache_sentiment(&holding.symbol, sentiment, config.sentiment.cache_ttl_secs).await?;
                sentiments.insert(holding.symbol.clone(), sentiment);
                log_action(&format!("{}: Fetched sentiment {:.2}", holding.symbol, sentiment), &config.environment);
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
    init_logger(&config.environment);
    let db = Database::new(&config.database.postgres_url, &config.redis.url).await?;
    let market_provider = MarketProvider::new(&config.exchanges[0].base_url, &config.exchanges[0].api_key);

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
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "portfolio" => portfolio_screen().expect("Portfolio screen failed"),
            "sentiment" => sentiment_screen().expect("Sentiment screen failed"),
            "market" => market_screen().expect("Market screen failed"),
            _ => eprintln!("Invalid subcommand. Use 'portfolio', 'sentiment', or 'market'."),
        }
    } else {
        main().expect("Main process failed");
    }
}