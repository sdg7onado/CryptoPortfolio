use crate::config::ExchangeConfig;
use crate::errors::PortfolioError;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct PriceResponse {
    price: f64,
}

pub trait Exchange {
    async fn fetch_price(&self, symbol: &str) -> Result<f64, PortfolioError>;
}

pub struct CoinGeckoExchange {
    client: Client,
    base_url: String,
}

impl CoinGeckoExchange {
    pub fn new(config: &ExchangeConfig) -> Self {
        CoinGeckoExchange {
            client: Client::new(),
            base_url: config.base_url.clone(),
        }
    }
}

impl Exchange for CoinGeckoExchange {
    async fn fetch_price(&self, symbol: &str) -> Result<f64, PortfolioError> {
        let url = format!(
            "{}/simple/price?ids={}&vs_currencies=usd",
            self.base_url, symbol
        );
        let resp = self
            .client
            .get(&url)
            .header("User-Agent", "crypto_portfolio/0.1")
            .send()
            .await
            .map_err(|e| PortfolioError::ExchangeError(e.to_string()))?;
        let json: HashMap<String, HashMap<String, f64>> = resp
            .json()
            .await
            .map_err(|e| PortfolioError::ExchangeError(e.to_string()))?;
        let price = json
            .get(symbol)
            .and_then(|m| m.get("usd"))
            .copied()
            .ok_or_else(|| PortfolioError::ExchangeError("Price not found".to_string()))?;
        Ok(price)
    }
}

// Add more exchanges (e.g., Binance) by implementing the Exchange trait
pub fn create_exchange(config: &ExchangeConfig) -> CoinGeckoExchange {
    match config.name.as_str() {
        "coingecko" => CoinGeckoExchange::new(config),
        _ => panic!("Unsupported exchange: {}", config.name),
    }
}

pub trait SentimentProvider {
    async fn fetch_sentiment(&self, symbol: &str) -> Result<f64, PortfolioError>;
    async fn fetch_detailed_sentiment(
        &self,
        symbol: &str,
    ) -> Result<DetailedSentiment, PortfolioError>;
}

pub struct LunarCrushProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl LunarCrushProvider {
    pub fn new(api_url: &str, api_key: &str) -> Self {
        LunarCrushProvider {
            client: reqwest::Client::new(),
            base_url: api_url.to_string(),
            api_key: api_key.to_string(),
        }
    }
}
#[derive(serde::Deserialize)]
struct SentimentResponse {
    sentiment: f64,
}

impl SentimentProvider for LunarCrushProvider {
    // async fn fetch_sentiment(&self, symbol: &str) -> Result<f64, PortfolioError> {
    //     let url = format!(
    //         "{}/topic/{}/sentiment?key={}",
    //         self.base_url, symbol, self.api_key
    //     );
    //     let resp = self
    //         .client
    //         .get(&url)
    //         .header("User-Agent", "crypto_portfolio/0.1")
    //         .send()
    //         .await
    //         .map_err(|e| PortfolioError::ExchangeError(e.to_string()))?;
    //     let data: SentimentResponse = resp
    //         .json()
    //         .await
    //         .map_err(|e| PortfolioError::ExchangeError(e.to_string()))?;
    //     Ok(data.sentiment)
    // }

    async fn fetch_sentiment(&self, symbol: &str) -> Result<f64, PortfolioError> {
        let detailed = self.fetch_detailed_sentiment(symbol).await?;
        Ok(detailed.current_value)
    }

    async fn fetch_detailed_sentiment(
        &self,
        symbol: &str,
    ) -> Result<DetailedSentiment, PortfolioError> {
        let url = format!(
            "{}/topic/{}/sentiment?key={}",
            self.base_url,
            symbol.to_lowercase(),
            self.api_key
        );
        let response = self.client.get(&url).send().await.map_err(|e| {
            PortfolioError::ApiError(format!("Failed to fetch sentiment for {}: {}", symbol, e))
        })?;

        let html_text = response.text().await.map_err(|e| {
            PortfolioError::ApiError(format!("Failed to parse HTML for {}: {}", symbol, e))
        })?;
        let html = Html::parse_document(&html_text);

        // Define CSS selectors (adjust based on actual HTML)
        let current_value_selector = Selector::parse(".current-value").unwrap();
        let daily_average_selector = Selector::parse(".daily-average").unwrap();
        let one_week_value_selector = Selector::parse(".one-week .value").unwrap();
        let one_week_change_selector = Selector::parse(".one-week .change").unwrap();
        let one_month_value_selector = Selector::parse(".one-month .value").unwrap();
        let one_month_change_selector = Selector::parse(".one-month .change").unwrap();
        let one_year_high_value_selector = Selector::parse(".one-year-high .value").unwrap();
        let one_year_high_date_selector = Selector::parse(".one-year-high .date").unwrap();
        let one_year_low_value_selector = Selector::parse(".one-year-low .value").unwrap();
        let one_year_low_date_selector = Selector::parse(".one-year-low .date").unwrap();
        let supportive_theme_selector = Selector::parse(".supportive-themes .theme").unwrap();
        let critical_theme_selector = Selector::parse(".critical-themes .theme").unwrap();
        let theme_name_selector = Selector::parse(".name").unwrap();
        let theme_weight_selector = Selector::parse(".weight").unwrap();
        let theme_description_selector = Selector::parse(".description").unwrap();
        let network_row_selector = Selector::parse(".network-engagement tr").unwrap();
        let network_selector = Selector::parse(".network").unwrap();
        let positive_selector = Selector::parse(".positive").unwrap();
        let positive_percentage_selector = Selector::parse(".positive-percentage").unwrap();
        let neutral_selector = Selector::parse(".neutral").unwrap();
        let neutral_percentage_selector = Selector::parse(".neutral-percentage").unwrap();
        let negative_selector = Selector::parse(".negative").unwrap();
        let negative_percentage_selector = Selector::parse(".negative-percentage").unwrap();

        // Helper function to parse percentage strings (e.g., "92%" -> 0.92)
        let parse_percentage = |text: &str| -> Result<f64, PortfolioError> {
            text.trim_end_matches('%')
                .parse::<f64>()
                .map(|v| v / 100.0)
                .map_err(|e| {
                    PortfolioError::ApiError(format!("Failed to parse percentage {}: {}", text, e))
                })
        };

        // Extract fields
        let current_value = html
            .select(&current_value_selector)
            .next()
            .ok_or_else(|| PortfolioError::ApiError("Missing current-value".to_string()))?
            .inner_html();
        let current_value = parse_percentage(&current_value)?;

        let daily_average = html
            .select(&daily_average_selector)
            .next()
            .ok_or_else(|| PortfolioError::ApiError("Missing daily-average".to_string()))?
            .inner_html();
        let daily_average = parse_percentage(&daily_average)?;

        let one_week_value = html
            .select(&one_week_value_selector)
            .next()
            .ok_or_else(|| PortfolioError::ApiError("Missing one-week value".to_string()))?
            .inner_html();
        let one_week_value = parse_percentage(&one_week_value)?;

        let one_week_change = html
            .select(&one_week_change_selector)
            .next()
            .ok_or_else(|| PortfolioError::ApiError("Missing one-week change".to_string()))?
            .inner_html();
        let one_week_change = parse_percentage(&one_week_change)?;

        let one_month_value = html
            .select(&one_month_value_selector)
            .next()
            .ok_or_else(|| PortfolioError::ApiError("Missing one-month value".to_string()))?
            .inner_html();
        let one_month_value = parse_percentage(&one_month_value)?;

        let one_month_change = html
            .select(&one_month_change_selector)
            .next()
            .ok_or_else(|| PortfolioError::ApiError("Missing one-month change".to_string()))?
            .inner_html();
        let one_month_change = parse_percentage(&one_month_change)?;

        let one_year_high = html
            .select(&one_year_high_value_selector)
            .next()
            .ok_or_else(|| PortfolioError::ApiError("Missing one-year-high value".to_string()))?
            .inner_html();
        let one_year_high = parse_percentage(&one_year_high)?;

        let one_year_high_date = html
            .select(&one_year_high_date_selector)
            .next()
            .ok_or_else(|| PortfolioError::ApiError("Missing one-year-high date".to_string()))?
            .inner_html();

        let one_year_low = html
            .select(&one_year_low_value_selector)
            .next()
            .ok_or_else(|| PortfolioError::ApiError("Missing one-year-low value".to_string()))?
            .inner_html();
        let one_year_low = parse_percentage(&one_year_low)?;

        let one_year_low_date = html
            .select(&one_year_low_date_selector)
            .next()
            .ok_or_else(|| PortfolioError::ApiError("Missing one-year-low date".to_string()))?
            .inner_html();

        let supportive_themes = html
            .select(&supportive_theme_selector)
            .map(|theme| {
                let name = theme
                    .select(&theme_name_selector)
                    .next()
                    .map(|n| n.inner_html())
                    .unwrap_or_default();
                let weight = theme
                    .select(&theme_weight_selector)
                    .next()
                    .map(|w| parse_percentage(&w.inner_html()).unwrap_or(0.0))
                    .unwrap_or(0.0);
                let description = theme
                    .select(&theme_description_selector)
                    .next()
                    .map(|d| d.inner_html())
                    .unwrap_or_default();
                Theme {
                    name,
                    weight,
                    description,
                }
            })
            .collect::<Vec<_>>();

        let critical_themes = html
            .select(&critical_theme_selector)
            .map(|theme| {
                let name = theme
                    .select(&theme_name_selector)
                    .next()
                    .map(|n| n.inner_html())
                    .unwrap_or_default();
                let weight = theme
                    .select(&theme_weight_selector)
                    .next()
                    .map(|w| parse_percentage(&w.inner_html()).unwrap_or(0.0))
                    .unwrap_or(0.0);
                let description = theme
                    .select(&theme_description_selector)
                    .next()
                    .map(|d| d.inner_html())
                    .unwrap_or_default();
                Theme {
                    name,
                    weight,
                    description,
                }
            })
            .collect::<Vec<_>>();

        let network_engagement = html
            .select(&network_row_selector)
            .map(|row| {
                let network = row
                    .select(&network_selector)
                    .next()
                    .map(|n| n.inner_html())
                    .unwrap_or_default();
                let positive = row
                    .select(&positive_selector)
                    .next()
                    .map(|p| p.inner_html().parse::<u64>().unwrap_or(0))
                    .unwrap_or(0);
                let positive_percentage = row
                    .select(&positive_percentage_selector)
                    .next()
                    .map(|p| parse_percentage(&p.inner_html()).unwrap_or(0.0))
                    .unwrap_or(0.0);
                let neutral = row
                    .select(&neutral_selector)
                    .next()
                    .map(|n| n.inner_html().parse::<u64>().unwrap_or(0))
                    .unwrap_or(0);
                let neutral_percentage = row
                    .select(&neutral_percentage_selector)
                    .next()
                    .map(|n| parse_percentage(&n.inner_html()).unwrap_or(0.0))
                    .unwrap_or(0.0);
                let negative = row
                    .select(&negative_selector)
                    .next()
                    .map(|n| n.inner_html().parse::<u64>().unwrap_or(0))
                    .unwrap_or(0);
                let negative_percentage = row
                    .select(&negative_percentage_selector)
                    .next()
                    .map(|n| parse_percentage(&n.inner_html()).unwrap_or(0.0))
                    .unwrap_or(0.0);
                (
                    network,
                    NetworkEngagement {
                        positive,
                        positive_percentage,
                        neutral,
                        neutral_percentage,
                        negative,
                        negative_percentage,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(DetailedSentiment {
            current_value,
            daily_average,
            one_week_value,
            one_week_change,
            one_month_value,
            one_month_change,
            one_year_high,
            one_year_high_date,
            one_year_low,
            one_year_low_date,
            supportive_themes,
            critical_themes,
            network_engagement,
        })
    }
}

pub fn create_sentiment_provider(api_url: &str, api_key: &str) -> LunarCrushProvider {
    LunarCrushProvider::new(api_url, api_key)
}

#[derive(Debug, Clone)]
pub struct DetailedSentiment {
    pub current_value: f64,
    pub daily_average: f64,
    pub one_week_value: f64,
    pub one_week_change: f64,
    pub one_month_value: f64,
    pub one_month_change: f64,
    pub one_year_high: f64,
    pub one_year_high_date: String,
    pub one_year_low: f64,
    pub one_year_low_date: String,
    pub supportive_themes: Vec<Theme>,
    pub critical_themes: Vec<Theme>,
    pub network_engagement: HashMap<String, NetworkEngagement>,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub weight: f64,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct NetworkEngagement {
    pub positive: u64,
    pub positive_percentage: f64,
    pub neutral: u64,
    pub neutral_percentage: f64,
    pub negative: u64,
    pub negative_percentage: f64,
}
