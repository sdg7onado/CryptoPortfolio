use crate::config::PortfolioConfig;
use crate::database::Database;
use crate::errors::PortfolioError;
use crate::exchange::Exchange;
use crate::exchange::SentimentProvider;
use crate::exchange::{BinanceExchange, LunarCrushProvider};
use crate::logger::log_action;
use crate::notification::Notifier;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Holding {
    pub symbol: String,
    pub quantity: f64,
    pub purchase_price: f64,
    pub stop_loss: f64,
}

#[derive(Debug)]
pub struct Portfolio {
    pub holdings: Vec<Holding>,
    pub cash: f64,
    pub config: PortfolioConfig,
}

impl Portfolio {
    pub fn new(config: PortfolioConfig) -> Self {
        Portfolio {
            holdings: vec![
                Holding {
                    symbol: "PHA".to_string(),
                    quantity: 250.0,
                    purchase_price: 0.20,
                    stop_loss: 0.16,
                },
                Holding {
                    symbol: "SUI".to_string(),
                    quantity: 10.0,
                    purchase_price: 3.00,
                    stop_loss: 2.40,
                },
                Holding {
                    symbol: "DUSK".to_string(),
                    quantity: 80.0,
                    purchase_price: 0.25,
                    stop_loss: 0.20,
                },
            ],
            cash: 0.0,
            config,
        }
    }

    pub async fn check_portfolio(
        &mut self,
        exchange: &BinanceExchange,
        sentiment_provider: &LunarCrushProvider,
        db: &Database,
        notifier: &Notifier,
        negative_threshold: f64, // Add parameter
        previous_value: f64,
        previous_prices: &HashMap<String, f64>,
        previous_sentiments: &HashMap<String, f64>,
    ) -> Result<f64, PortfolioError> {
        let mut current_prices = HashMap::new();
        let mut current_sentiments = HashMap::new();

        let mut to_sell = Vec::new();
        for holding in self.holdings.iter() {
            let current_price = exchange.fetch_price(&holding.symbol).await?;
            let sentiment = sentiment_provider.fetch_sentiment(&holding.symbol).await?;
            current_prices.insert(holding.symbol.clone(), current_price);
            current_sentiments.insert(holding.symbol.clone(), sentiment);

            // Check stop-loss
            if current_price < holding.stop_loss || sentiment < negative_threshold {
                to_sell.push((
                    holding.symbol.clone(),
                    holding.quantity,
                    current_price,
                    sentiment,
                ));
            }
        }

        for (symbol, quantity, current_price, sentiment) in to_sell {
            let proceeds = self.sell_holding(&symbol, exchange, db, notifier).await?;
            let _ = log_action(
                &format!(
                    "Sold {} {} at ${:.2} (sentiment: {:.2}) for ${:.2}",
                    quantity, symbol, current_price, sentiment, proceeds
                ),
                None,
            );
            notifier.notify_significant_action(&format!(
                "{}: Negative sentiment triggered at ${:.2} (sentiment: {:.2}), sold {} tokens for ${:.2}.",
                symbol, current_price, sentiment, quantity, proceeds
            )).await?;
        }

        let total_value = self.get_value(exchange).await?;
        notifier
            .notify_major_change(
                self,
                previous_value,
                total_value,
                previous_prices,
                &current_prices,
            )
            .await?;

        for (symbol, sentiment) in &current_sentiments {
            if let Some(prev_sentiment) = previous_sentiments.get(symbol) {
                notifier
                    .notify_sentiment_change(symbol, *prev_sentiment, *sentiment)
                    .await?;
            }
        }

        Ok(total_value)
    }

    pub async fn get_value(&self, exchange: &BinanceExchange) -> Result<f64, PortfolioError> {
        let mut total_value = self.cash;
        for holding in &self.holdings {
            let current_price = exchange.fetch_price(&holding.symbol).await?;
            total_value += holding.quantity * current_price;
        }
        Ok(total_value)
    }

    pub async fn sell_holding(
        &mut self,
        symbol: &str,
        exchange: &BinanceExchange,
        db: &Database,
        notifier: &Notifier,
    ) -> Result<f64, PortfolioError> {
        if let Some(index) = self.holdings.iter().position(|h| h.symbol == symbol) {
            let holding = self.holdings.remove(index);
            let price = exchange.fetch_price(&holding.symbol).await?;
            let proceeds = holding.quantity * price;
            self.cash += proceeds;
            db.log_trade(&holding.symbol, holding.quantity, price, "sell")
                .await?;
            let _ = log_action(
                &format!(
                    "Sold {} {} at ${:.2} for ${:.2}",
                    holding.quantity, holding.symbol, price, proceeds
                ),
                None,
            );
            notifier
                .notify_significant_action(&format!(
                    "Sold {} {} at ${:.2} for ${:.2}",
                    holding.quantity, holding.symbol, price, proceeds
                ))
                .await?;
            Ok(proceeds)
        } else {
            Err(PortfolioError::ExchangeError(format!(
                "Holding {} not found",
                symbol
            )))
        }
    }
}
