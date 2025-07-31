use crate::exchange::Exchange;
use crate::database::{Database, Trade};
use crate::display::display_portfolio;
use crate::errors::PortfolioError;
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct Holding {
    pub symbol: String,
    pub quantity: f64,
    pub purchase_price: f64,
    pub stop_loss: f64,
}

#[derive(Clone, Debug)]
pub struct Portfolio {
    pub holdings: Vec<Holding>,
    pub cash: f64,
}

#[async_trait]
pub trait PortfolioManager {
    async fn check_stop_loss(&mut self, exchange: &dyn Exchange, db: &Database) -> Result<Vec<String>, PortfolioError>;
    async fn rebalance(&mut self, exchange: &dyn Exchange, db: &Database, max_allocation: f64) -> Result<Vec<String>, PortfolioError>;
    async fn get_value(&self, exchange: &dyn Exchange) -> Result<f64, PortfolioError>;
}

#[async_trait]
impl PortfolioManager for Portfolio {
    async fn check_stop_loss(&mut self, exchange: &dyn Exchange, db: &Database) -> Result<Vec<String>, PortfolioError> {
        let mut actions = Vec::new();
        for holding in self.holdings.iter_mut() {
            let current_price = exchange.fetch_price(&holding.symbol).await?;
            if current_price <= holding.stop_loss {
                let sale_value = holding.quantity * current_price;
                self.cash += sale_value;
                actions.push(format!(
                    "{}: Stop-loss triggered at ${:.2}, sold {} tokens for ${:.2}",
                    holding.symbol, current_price, holding.quantity, sale_value
                ));
                db.log_trade(Trade {
                    symbol: holding.symbol.clone(),
                    quantity: holding.quantity,
                    price: current_price,
                    action: "sell".to_string(),
                    timestamp: chrono::Utc::now(),
                })
                .await?;
                holding.quantity = 0.0;
            }
        }
        self.holdings.retain(|h| h.quantity > 0.0);
        Ok(actions)
    }

    async fn rebalance(&mut self, exchange: &dyn Exchange, db: &Database, max_allocation: f64) -> Result<Vec<String>, PortfolioError> {
        let mut actions = Vec::new();
        let total_value = self.get_value(exchange).await?;
        for holding in self.holdings.iter_mut() {
            let price = exchange.fetch_price(&holding.symbol).await?;
            let holding_value = holding.quantity * price;
            if holding_value / total_value > max_allocation {
                let excess = holding_value - (total_value * max_allocation);
                let sell_qty = excess / price;
                let sale_value = sell_qty * price;
                self.cash += sale_value;
                holding.quantity -= sell_qty;
                actions.push(format!(
                    "{}: Rebalancing, sold {} tokens for ${:.2}",
                    holding.symbol, sell_qty, sale_value
                ));
                db.log_trade(Trade {
                    symbol: holding.symbol.clone(),
                    quantity: sell_qty,
                    price,
                    action: "sell".to_string(),
                    timestamp: chrono::Utc::now(),
                })
                .await?;
            }
        }
        Ok(actions)
    }

    async fn get_value(&self, exchange: &dyn Exchange) -> Result<f64, PortfolioError> {
        let mut total_value = self.cash;
        for holding in &self.holdings {
            let price = exchange.fetch_price(&holding.symbol).await?;
            total_value += holding.quantity * price;
        }
        Ok(total_value)
    }
}