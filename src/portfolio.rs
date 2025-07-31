use crate::exchange::{Exchange, SentimentProvider};
use crate::database::Database;
use crate::errors::PortfolioError;
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Holding {
    pub symbol: String,
    pub quantity: f64,
    pub purchase_price: f64,
    pub stop_loss: f64,
}

pub struct Portfolio {
    pub holdings: Vec<Holding>,
    pub cash: f64,
    pub previous_value: f64, // New: Track previous portfolio value
    pub previous_prices: HashMap<String, f64>, // New: Track previous prices
    pub previous_sentiments: HashMap<String, f64>, // New: Track previous sentiments
}

#[async_trait]
pub trait PortfolioManager {
    async fn check_stop_loss(
        &mut self,
        exchange: &dyn Exchange,
        sentiment_provider: &dyn SentimentProvider,
        db: &Database,
        sentiment_threshold: f64,
    ) -> Result<Vec<String>, PortfolioError>;
    async fn rebalance(
        &mut self,
        exchange: &dyn Exchange,
        sentiment_provider: &dyn SentimentProvider,
        db: &Database,
        max_allocation: f64,
        sentiment_threshold: f64,
    ) -> Result<Vec<String>, PortfolioError>;
    async fn get_value(&self, exchange: &dyn Exchange) -> Result<f64, PortfolioError>;
}

impl Portfolio {
    pub fn new() -> Self {
        Portfolio {
            holdings: vec![],
            cash: 0.0,
            previous_value: 0.0,
            previous_prices: HashMap::new(),
            previous_sentiments: HashMap::new(),
        }
    }
}

#[async_trait]
impl PortfolioManager for Portfolio {
    async fn check_stop_loss(
        &mut self,
        exchange: &dyn Exchange,
        sentiment_provider: &dyn SentimentProvider,
        db: &Database,
        sentiment_threshold: f64,
    ) -> Result<Vec<String>, PortfolioError> {
        let mut actions = vec![];
        let mut i = 0;
        while i < self.holdings.len() {
            let holding = &self.holdings[i];
            let current_price = exchange.fetch_price(&holding.symbol).await?;
            let sentiment = sentiment_provider.fetch_sentiment(&holding.symbol).await?;
            if current_price <= holding.stop_loss || sentiment <= sentiment_threshold {
                let sale_value = holding.quantity * current_price;
                self.cash += sale_value;
                actions.push(format!(
                    "{}: {} triggered at ${:.2} (sentiment: {:.2}), sold {} tokens for ${:.2}",
                    holding.symbol, if current_price <= holding.stop_loss { "Stop-loss" } else { "Negative sentiment" },
                    current_price, sentiment, holding.quantity, sale_value
                ));
                self.holdings.remove(i);
            } else {
                i += 1;
            }
        }
        Ok(actions)
    }

    async fn rebalance(
        &mut self,
        exchange: &dyn Exchange,
        sentiment_provider: &dyn SentimentProvider,
        db: &Database,
        max_allocation: f64,
        sentiment_threshold: f64,
    ) -> Result<Vec<String>, PortfolioError> {
        let mut actions = vec![];
        let total_value = self.get_value(exchange).await?;
        for holding in &self.holdings {
            let current_price = exchange.fetch_price(&holding.symbol).await?;
            let sentiment = sentiment_provider.fetch_sentiment(&holding.symbol).await?;
            let holding_value = holding.quantity * current_price;
            let allocation = holding_value / total_value;
            if allocation > max_allocation && sentiment < sentiment_threshold {
                let excess_value = holding_value - (max_allocation * total_value);
                let sell_quantity = excess_value / current_price;
                let sale_value = sell_quantity * current_price;
                self.cash += sale_value;
                actions.push(format!(
                    "{}: Rebalancing, sold {} tokens for ${:.2} (allocation: {:.2}%, sentiment: {:.2})",
                    holding.symbol, sell_quantity, sale_value, allocation * 100.0, sentiment
                ));
                // Update holding quantity
                if let Some(h) = self.holdings.iter_mut().find(|h| h.symbol == holding.symbol) {
                    h.quantity -= sell_quantity;
                }
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