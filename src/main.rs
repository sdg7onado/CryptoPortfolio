use dotenv::dotenv;
use tokio::time::{sleep, Duration};
use crate::config::load_config;
use crate::exchange::create_exchange;
use crate::portfolio::{Portfolio, PortfolioManager};
use crate::database::Database;
use crate::logger::{init_logger, log_action};
use crate::display::display_portfolio;
use crate::errors::PortfolioError;

mod config;
mod exchange;
mod portfolio;
mod database;
mod logger;
mod display;
mod errors;

#[tokio::main]
async fn main() -> Result<(), PortfolioError> {
    dotenv().ok();
    let config = load_config()?;
    init_logger(&config.environment);

    let db = Database::new(&config.database.postgres_url, &config.redis.url).await?;
    let exchange = create_exchange(&config.exchanges[0]); // Use first exchange; extend for multi-exchange
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
    };

    loop {
        // Fetch and cache prices
        for holding in &portfolio.holdings {
            if let Some(cached_price) = db.get_cached_price(&holding.symbol).await? {
                log_action(&format!("{}: Using cached price ${:.2}", holding.symbol, cached_price), &config.environment);
            } else {
                let price = exchange.fetch_price(&holding.symbol).await?;
                db.cache_price(&holding.symbol, price).await?;
                log_action(&format!("{}: Fetched price ${:.2}", holding.symbol, price), &config.environment);
            }
        }

        // Check stop-losses
        let stop_loss_actions = portfolio.check_stop_loss(&*exchange, &db).await?;
        for action in stop_loss_actions {
            log_action(&action, &config.environment);
            println!("{}", action);
        }

        // Rebalance if needed
        let rebalance_actions = portfolio.rebalance(&*exchange, &db, config.portfolio.max_allocation).await?;
        for action in rebalance_actions {
            log_action(&action, &config.environment);
            println!("{}", action);
        }

        // Display portfolio
        let total_value = portfolio.get_value(&*exchange).await?;
        display_portfolio(&portfolio, total_value);
        log_action(&format!("Portfolio value: ${:.2}", total_value), &config.environment);

        // Wait for next check
        sleep(Duration::from_secs(config.portfolio.check_interval_secs)).await;
    }
}