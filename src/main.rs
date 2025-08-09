use crate::config::load_config;
use crate::database::Database;
use crate::display::{display_portfolio, display_sentiment_screen};
use crate::errors::PortfolioError;
use crate::exchange::{create_exchange, create_sentiment_provider, Exchange, SentimentProvider};
use crate::logger::{init_logger, log_action};
use crate::market::{display_market_screen, MarketProvider};
use crate::notification::Notifier;
use crate::portfolio::Portfolio;
use dotenv::dotenv;
use std::collections::HashMap;
use std::process::{Child, Command};
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

async fn portfolio_screen() -> Result<(), PortfolioError> {
    let config = load_config()?;
    init_logger(&config.environment)?;
    let env = Some(config.environment.as_str());
    let db = Database::new(&config.database.postgres_url, &config.redis.url).await?;
    let exchange = create_exchange(&config.exchanges[0]);
    let sentiment_provider =
        create_sentiment_provider(&config.sentiment.api_url, &config.sentiment.api_key);
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
                    env,
                )?;
                current_prices.insert(holding.symbol.clone(), cached_price);
            } else {
                let price = exchange.fetch_price(&holding.symbol).await?;
                db.cache_price(&holding.symbol, price).await?;
                log_action(
                    &format!("{}: Fetched price ${:.2}", holding.symbol, price),
                    env,
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
                    env,
                )?;
            } else {
                let sentiment = sentiment_provider.fetch_sentiment(&holding.symbol).await?;
                db.cache_sentiment(&holding.symbol, sentiment, config.sentiment.cache_ttl_secs)
                    .await?;
                sentiments.insert(holding.symbol.clone(), sentiment);
                log_action(
                    &format!("{}: Fetched sentiment {:.2}", holding.symbol, sentiment),
                    env,
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
        log_action(&format!("Portfolio value: ${:.2}", total_value), env)?;

        sleep(Duration::from_secs(config.portfolio.check_interval_secs)).await;
    }
}

async fn sentiment_screen() -> Result<(), PortfolioError> {
    let config = load_config()?;
    init_logger(&config.environment)?;
    let env = Some(config.environment.as_str());
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
                    env,
                )?;
            } else {
                let sentiment = sentiment_provider.fetch_sentiment(&holding.symbol).await?;
                db.cache_sentiment(&holding.symbol, sentiment, config.sentiment.cache_ttl_secs)
                    .await?;
                sentiments.insert(holding.symbol.clone(), sentiment);
                log_action(
                    &format!("{}: Fetched sentiment {:.2}", holding.symbol, sentiment),
                    env,
                )?;
            }
        }

        display_sentiment_screen(
            &portfolio,
            &sentiments,
            &db,
            &sentiment_provider,
            config.sentiment.positive_threshold,
            config.sentiment.negative_threshold,
            config.display.use_colors,
        )
        .await?;

        sleep(Duration::from_secs(config.display.sentiment_refresh_secs)).await;
    }
}

async fn market_screen() -> Result<(), PortfolioError> {
    let config = load_config()?;
    init_logger(&config.environment)?;
    let db = Database::new(&config.database.postgres_url, &config.redis.url).await?;
    let exchange = create_exchange(&config.exchanges[0]);
    let market_provider = MarketProvider::new(
        &config.marketprovider.base_url,
        &config.marketprovider.api_key,
        &exchange,
    );

    loop {
        display_market_screen(
            &market_provider,
            &config.market.pinned_symbols,
            &config.market.sort_by,
            config.display.use_colors,
        )
        .await?;

        sleep(Duration::from_secs(config.market.refresh_secs)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), PortfolioError> {
    dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "portfolio" => portfolio_screen().await,
            "sentiment" => sentiment_screen().await,
            "market" => market_screen().await,
            _ => {
                eprintln!("Invalid subcommand. Use 'portfolio', 'sentiment', or 'market'.");
                Ok(())
            }
        }
    } else {
        let config = load_config()?;
        init_logger(&config.environment)?;

        if config.environment == "dev" {
            println!("Running in development mode. Use 'cargo run -- <subcommand>' to start a specific screen.");

            // Run screens directly in development for easier debugging
            println!("Running all screens in a single process for debugging. Use Ctrl+C to stop.");
            let portfolio_handle = tokio::spawn(portfolio_screen());
            let sentiment_handle = tokio::spawn(sentiment_screen());
            let market_handle = tokio::spawn(market_screen());

            // Wait for Ctrl+C to terminate
            tokio::select! {
                _ = portfolio_handle => eprintln!("Portfolio screen terminated"),
                _ = sentiment_handle => eprintln!("Sentiment screen terminated"),
                _ = market_handle => eprintln!("Market screen terminated"),
                _ = tokio::signal::ctrl_c() => println!("Received Ctrl+C, shutting down"),
            };
            Ok(())
        } else {
            println!("Running in production mode. Use 'target/release/crypto_portfolio <subcommand>' to start a specific screen.");

            // Use pre-built binary to avoid file locks
            let executable = if cfg!(target_os = "windows") {
                "target\\release\\crypto_portfolio.exe"
            } else {
                "./target/release/crypto_portfolio"
            };

            // Detect terminal emulator for Linux
            let (terminal_cmd, terminal_args) = if cfg!(target_os = "windows") {
                ("cmd", vec!["/C", "start", "cmd", "/K", executable])
            } else {
                let terminals = [
                    ("gnome-terminal", vec!["--", executable]),
                    ("konsole", vec!["-e", executable]),
                    ("xterm", vec!["-e", executable]),
                ];
                terminals
                    .into_iter()
                    .find(|(cmd, _)| Command::new(cmd).arg("--version").output().is_ok())
                    .unwrap_or_else(|| {
                        eprintln!("No terminal emulator found (gnome-terminal, konsole, xterm). Falling back to xterm.");
                        ("xterm", vec!["-e", executable])
                    })
            };

            // Store child processes for cleanup
            let mut children: Vec<Child> = Vec::new();

            // Spawn console windows for each screen
            for screen in ["portfolio", "sentiment", "market"] {
                match Command::new(terminal_cmd)
                    .args(&terminal_args)
                    .arg(screen)
                    .spawn()
                {
                    Ok(child) => {
                        let pid = child.id();
                        println!("Spawned {} screen (PID: {})", screen, pid);
                        children.push(child);
                    }
                    Err(e) => eprintln!("Failed to spawn {} screen: {}", screen, e),
                }
            }

            // Wait for Ctrl+C to terminate
            ctrlc::set_handler({
                let mut children = children;
                move || {
                    println!("Received Ctrl+C, terminating child processes...");
                    for child in children.iter_mut() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                    std::process::exit(0);
                }
            })
            .expect("Failed to set Ctrl+C handler");

            // Keep the main process alive
            std::thread::sleep(std::time::Duration::from_secs(3600));
            Ok(())
        }
    }
}
