use crate::config::ExchangeConfig;
use crate::errors::PortfolioError;
use crate::logger::log_action;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::collections::HashMap;

pub trait Exchange {
    async fn fetch_price(&self, symbol: &str) -> Result<f64, PortfolioError>;
}

#[derive(Debug, Clone)]
pub struct DetailedSentiment {
    pub current_value: f64,
    pub daily_average: f64,
    pub one_week_value: f64,
    pub one_week_change: f64,
    pub one_month_value: f64,
    pub one_month_change: f64,
    pub six_months_value: f64,
    pub six_months_change: f64,
    pub one_year_value: f64,
    pub one_year_change: f64,
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
    pub positive: String,
    pub positive_percentage: f64,
    pub neutral: String,
    pub neutral_percentage: f64,
    pub negative: String,
    pub negative_percentage: f64,
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

impl SentimentProvider for LunarCrushProvider {
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

        let _ = log_action(
            &format!(
                "Fetched sentiment for symbol: {} \n Raw: {}",
                symbol, html_text
            ),
            None,
        );

        let html = Html::parse_document(&html_text);

        let _ = log_action(
            &format!(
                "Fetched sentiment for symbol: {} \n Pre: {}",
                symbol,
                html.html()
            ),
            None,
        );

        let html = Html::parse_document(&html.html());

        // Extract text from <pre> tag
        let pre_selector = Selector::parse("body").unwrap();
        let pre_text = html
            .select(&pre_selector)
            .next()
            .ok_or_else(|| PortfolioError::ApiError("Missing <pre> tag in HTML".to_string()))?
            .inner_html();

        // Split pre_text into lines for parsing
        let lines: Vec<&str> = pre_text.lines().collect();

        let mut current_value = 0.0;
        let mut daily_average = 0.0;
        let mut one_week_value = 0.0;
        let mut one_week_change = 0.0;
        let mut one_month_value = 0.0;
        let mut one_month_change = 0.0;
        let mut six_months_value = 0.0;
        let mut six_months_change = 0.0;
        let mut one_year_value = 0.0;
        let mut one_year_change = 0.0;
        let mut one_year_high = 0.0;
        let mut one_year_high_date = String::new();
        let mut one_year_low = 0.0;
        let mut one_year_low_date = String::new();
        let mut supportive_themes = Vec::new();
        let mut critical_themes = Vec::new();
        //let mut network_engagement: HashMap<String, NetworkEngagement> = HashMap::new();

        let parse_percentage = |text: &str| -> f64 {
            text.trim_end_matches('%')
                .trim()
                .parse::<f64>()
                .unwrap_or(0.0)
                / 100.0
        };

        let parse_number = |text: &str| -> String { text.replace(",", "").trim().to_string() };

        let mut in_supportive_themes = false;
        let mut in_critical_themes = false;
        let mut in_network_table = false;
        let mut network_table_lines = Vec::new();

        for line in lines {
            let line_trim = line.trim();
            if line_trim.starts_with("**Current Value**:") {
                current_value = parse_percentage(&line_trim[18..]);
            } else if line_trim.starts_with("**Daily Average**:") {
                daily_average = parse_percentage(&line_trim[18..]);
            } else if line_trim.starts_with("**1 Week**:") {
                let parts: Vec<&str> = line_trim[11..].split_whitespace().collect();
                one_week_value = parse_percentage(parts[0]);
                one_week_change = parse_percentage(&parts[1][0..parts[1].len() - 1]);
            } else if line_trim.starts_with("**1 Month**:") {
                let parts: Vec<&str> = line_trim[12..].split_whitespace().collect();
                one_month_value = parse_percentage(parts[0]);
                one_month_change = parse_percentage(&parts[1][0..parts[1].len() - 1]);
            } else if line_trim.starts_with("**6 Months**:") {
                let parts: Vec<&str> = line_trim[13..].split_whitespace().collect();
                six_months_value = parse_percentage(parts[0]);
                six_months_change = parse_percentage(&parts[1][0..parts[1].len() - 1]);
            } else if line_trim.starts_with("**1 Year**:") {
                let parts: Vec<&str> = line_trim[11..].split_whitespace().collect();
                one_year_value = parse_percentage(parts[0]);
                one_year_change = parse_percentage(&parts[1][0..parts[1].len() - 1]);
            } else if line_trim.starts_with("**1-Year High**:") {
                let parts: Vec<&str> = line_trim[16..].split(" on ").collect();
                one_year_high = parse_percentage(parts[0]);
                one_year_high_date = parts[1].to_string();
            } else if line_trim.starts_with("**1-Year Low**:") {
                let parts: Vec<&str> = line_trim[15..].split(" on ").collect();
                one_year_low = parse_percentage(parts[0]);
                one_year_low_date = parts[1].to_string();
            } else if line_trim.starts_with("**Most Supportive Themes**") {
                in_supportive_themes = true;
                in_critical_themes = false;
            } else if line_trim.starts_with("**Most Critical Themes**") {
                in_supportive_themes = false;
                in_critical_themes = true;
            } else if line_trim.starts_with("Network engagement breakdown:") {
                in_supportive_themes = false;
                in_critical_themes = false;
                in_network_table = true;
            } else if in_supportive_themes {
                if line_trim.starts_with("- **") {
                    let theme_end = line_trim.find(":**").unwrap_or(line_trim.len());
                    let name = &line_trim[4..theme_end - 3];
                    let weight_start = line_trim.find("(").unwrap_or(line_trim.len());
                    let weight_end = line_trim.find("%)").unwrap_or(line_trim.len());
                    let weight_str = &line_trim[weight_start + 1..weight_end];
                    let weight = parse_percentage(weight_str);
                    let description = &line_trim[weight_end + 3..].trim();
                    supportive_themes.push(Theme {
                        name: name.to_string(),
                        weight,
                        description: description.to_string(),
                    });
                }
            } else if in_critical_themes {
                if line_trim.starts_with("- **") {
                    let theme_end = line_trim.find(":**").unwrap_or(line_trim.len());
                    let name = &line_trim[4..theme_end - 3];
                    let weight_start = line_trim.find("(").unwrap_or(line_trim.len());
                    let weight_end = line_trim.find("%)").unwrap_or(line_trim.len());
                    let weight_str = &line_trim[weight_start + 1..weight_end];
                    let weight = parse_percentage(weight_str);
                    let description = &line_trim[weight_end + 3..].trim();
                    critical_themes.push(Theme {
                        name: name.to_string(),
                        weight,
                        description: description.to_string(),
                    });
                }
            } else if in_network_table {
                if line_trim.starts_with("|") {
                    network_table_lines.push(line_trim.to_string());
                }
            }
        }

        // Parse network engagement table
        let mut network_engagement = HashMap::new();
        for line in network_table_lines.iter().skip(1) {
            // Skip header
            let cells: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
            if cells.len() == 8 {
                // Including empty cells
                let network = cells[1];
                let positive = parse_number(cells[2]);
                let positive_percentage = parse_percentage(cells[3]);
                let neutral = parse_number(cells[4]);
                let neutral_percentage = parse_percentage(cells[5]);
                let negative = parse_number(cells[6]);
                let negative_percentage = parse_percentage(cells[7]);
                network_engagement.insert(
                    network.to_string(),
                    NetworkEngagement {
                        positive,
                        positive_percentage,
                        neutral,
                        neutral_percentage,
                        negative,
                        negative_percentage,
                    },
                );
            }
        }

        Ok(DetailedSentiment {
            current_value,
            daily_average,
            one_week_value,
            one_week_change,
            one_month_value,
            one_month_change,
            six_months_value,
            six_months_change,
            one_year_value,
            one_year_change,
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

pub struct BinanceExchange {
    client: Client,
    pub api_key: String,
    pub api_secret: String,
    pub api_url: String,
    symbol_map: HashMap<String, String>, // Maps app symbols (e.g., "PHA") to Binance symbols (e.g., "PHAUSDT")
}

impl BinanceExchange {
    pub fn new(
        api_url: &str,
        api_key: &str,
        api_secret: &str,
        symbol_map: HashMap<String, String>,
    ) -> Self {
        BinanceExchange {
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
            client: Client::new(),
            symbol_map,
        }
    }
}

impl Exchange for BinanceExchange {
    async fn fetch_price(&self, symbol: &str) -> Result<f64, PortfolioError> {
        let binance_symbol = self.symbol_map.get(symbol).ok_or_else(|| {
            PortfolioError::ApiError(format!("Symbol {} not supported by Binance", symbol))
        })?;

        let url = format!(
            "{}/api/v3/ticker/price?symbol={}",
            self.api_url, binance_symbol
        );
        let response = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| {
                PortfolioError::ApiError(format!("Failed to fetch price for {}: {}", symbol, e))
            })?;

        #[derive(Deserialize)]
        struct BinancePrice {
            symbol: String,
            price: String,
        }

        let price_data: BinancePrice = response.json().await.map_err(|e| {
            PortfolioError::ApiError(format!(
                "Failed to parse Binance price JSON for {}: {}",
                symbol, e
            ))
        })?;

        let price = price_data.price.parse::<f64>().map_err(|e| {
            PortfolioError::ApiError(format!("Failed to parse price for {}: {}", symbol, e))
        })?;

        Ok(price)
    }
}

pub fn create_exchange(config: &ExchangeConfig) -> BinanceExchange {
    match config.name.as_str() {
        "binance" => {
            // Define symbol mappings for Binance
            let mut symbol_map = HashMap::new();
            symbol_map.insert("PHA".to_string(), "PHAUSDT".to_string());
            symbol_map.insert("SUI".to_string(), "SUIUSDT".to_string());
            symbol_map.insert("DUSK".to_string(), "DUSKUSDT".to_string());

            BinanceExchange::new(
                &config.base_url,
                &config.api_key,
                &config.api_secret,
                symbol_map,
            )
        }
        _ => {
            let _ = log_action(&format!("Unsupported exchange: {}", config.name), None);
            panic!("Unsupported exchange: {}", config.name)
        }
    }
}

pub trait SentimentProvider {
    async fn fetch_sentiment(&self, symbol: &str) -> Result<f64, PortfolioError>;
    async fn fetch_detailed_sentiment(
        &self,
        symbol: &str,
    ) -> Result<DetailedSentiment, PortfolioError>;
}

#[derive(serde::Deserialize)]
struct SentimentResponse {
    sentiment: f64,
}

#[derive(Deserialize)]
struct PriceResponse {
    price: f64,
}
